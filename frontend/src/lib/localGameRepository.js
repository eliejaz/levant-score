import { evaluateRound, getGamePresets } from './rulesEngine.js'
import { calculateStandings } from './standings.js'

const STORAGE_KEY='levant-score:local:v2'
const LEGACY_STORAGE_KEY='levant-score:local:v1'
const HISTORY_PAGE_SIZE=20
const HISTORY_GAME_LIMIT=10
const EMPTY_STATE={version:2,currentGame:null,historyGames:[]}

export const localGameRepository={
  async getPresets(){ return clone(await getGamePresets()) },
  async listGames({status='ACTIVE',page=1,size=6}={}){
    const state=readState()
    const games=[state.currentGame,...state.historyGames].filter(Boolean)
    const filtered=games.filter(game=>game.status===status).sort((a,b)=>b.updatedAt.localeCompare(a.updatedAt))
    const start=(page-1)*size
    return {items:filtered.slice(start,start+size).map(summary),page,size,totalItems:filtered.length,totalPages:Math.ceil(filtered.length/size),stats:{allTimeRounds:games.reduce((sum,game)=>sum+game.roundCount,0),activeGames:games.filter(game=>game.status==='ACTIVE').length,finishedGames:games.filter(game=>game.status==='FINISHED').length}}
  },
  async getGame(id){ return publicGame(requireGame(readState(),id)) },
  async listRounds(id,{beforeRound,limit=HISTORY_PAGE_SIZE}={}){
    const game=requireGame(readState(),id)
    const eligible=game.rounds.filter(round=>beforeRound==null||round.roundNumber<beforeRound)
    return {items:eligible.slice(0,limit).map(publicRound),hasMore:eligible.length>limit}
  },
  async createGame(request){
    const state=readState()
    if(state.currentGame) throw conflict('Finish or delete the current game before starting another one.')
    const presets=await getGamePresets()
    const preset=presets.find(item=>item.code===request.presetCode)||presets.at(-1)
    validateCreate(request,preset)
    const id=uuid(), now=new Date().toISOString()
    const teamMode=request.teamMode??preset.teamMode
    const players=request.players.map((player,index)=>({id:uuid(),name:player.name.trim(),teamNumber:teamMode?Number(player.teamNumber):null,seatOrder:index,totalScore:0}))
    const game={id,title:request.title.trim(),presetCode:preset.code,presetName:preset.code==='custom'?(request.customPresetName?.trim()||preset.name):preset.name,teamMode,scoreDirection:request.scoreDirection??preset.direction,targetScore:nullableNumber(request.targetScore??preset.defaultTarget),thresholdRule:request.thresholdRule??preset.thresholdRule,scoreMode:request.scoreMode??preset.scoreMode,roundLimit:nullableNumber(request.roundLimit??preset.roundLimit),roundCount:0,status:'ACTIVE',outcome:null,players,rounds:[],createdAt:now,updatedAt:now}
    state.currentGame=game
    writeState(state)
    return publicGame(game)
  },
  async deleteGame(id){
    const state=readState()
    if(state.currentGame?.id===id) state.currentGame=null
    else {const before=state.historyGames.length;state.historyGames=state.historyGames.filter(game=>game.id!==id);if(state.historyGames.length===before)throw notFound()}
    writeState(state)
  },
  async rematchGame(id){
    const state=readState(), source=requireGame(state,id)
    if(state.currentGame&&state.currentGame.id!==id) throw conflict('Finish or delete the current game before starting a rematch.')
    const now=new Date().toISOString(), nextId=uuid()
    const next={...clone(source),id:nextId,status:'ACTIVE',outcome:null,roundCount:0,rounds:[],players:source.players.map(player=>({...player,id:uuid(),totalScore:0})),createdAt:now,updatedAt:now}
    if(source.status==='FINISHED') addHistory(state,source)
    state.currentGame=next
    writeState(state)
    return publicGame(next)
  },
  async addRound(id,request){
    const state=readState(), game=requireGame(state,id)
    if(game.status==='FINISHED') throw conflict('Reopen the game before adding a round')
    if(!request.scores?.length) throw bad('scores must not be empty')
    if((request.note?.length||0)>240) throw bad('note must be at most 240 characters')
    const evaluation=await evaluateRound(game,request)
    const now=new Date().toISOString(), roundId=uuid()
    const deltas=evaluation.scores.map(score=>({playerId:score.playerId,points:score.points}))
    for(const delta of deltas) game.players.find(player=>player.id===delta.playerId).totalScore+=delta.points
    game.roundCount+=1
    const round={id:roundId,roundNumber:game.roundCount,ruleCode:request.ruleCode||null,note:request.note?.trim()||null,createdAt:now,inputData:clone(request),scores:deltas.map(delta=>roundScore(game,delta)),_deltas:deltas}
    game.rounds.unshift(round)
    game.status=evaluation.finished?'FINISHED':'ACTIVE'
    game.outcome=evaluation.outcome??null
    game.updatedAt=now
    if(game.status==='FINISHED'){state.currentGame=null;addHistory(state,game)}else state.currentGame=game
    writeState(state)
    return mutation(game,{round:publicRound(round)})
  },
  async updateRound(id,roundId,request){
    const state=readState(), source=requireGame(state,id)
    if(!request.scores?.length) throw bad('scores must not be empty')
    if((request.note?.length||0)>240) throw bad('note must be at most 240 characters')
    if(!source.rounds.some(round=>round.id===roundId)) throw notFound()
    const editedNumber=source.rounds.find(round=>round.id===roundId).roundNumber
    const originals=source.rounds.filter(round=>round.roundNumber>=editedNumber).slice().sort((a,b)=>a.roundNumber-b.roundNumber)
    const game=clone(source)
    for(const original of originals)for(const delta of original._deltas||[])game.players.find(player=>player.id===delta.playerId).totalScore-=delta.points
    game.rounds=game.rounds.filter(round=>round.roundNumber<editedNumber);game.roundCount=editedNumber-1;game.status='ACTIVE';game.outcome=null
    for(let index=0;index<originals.length;index+=1){
      const original=originals[index]
      const input=original.id===roundId?clone(request):clone(original.inputData)
      if(!input)throw conflict('An older round cannot be replayed. Undo it and enter it again before editing earlier rounds.')
      const evaluation=await evaluateRound(game,input)
      if(evaluation.finished&&index<originals.length-1)throw conflict('This change would end the game before a later round. Undo the later rounds first.')
      const deltas=evaluation.scores.map(score=>({playerId:score.playerId,points:score.points}))
      for(const delta of deltas) game.players.find(player=>player.id===delta.playerId).totalScore+=delta.points
      game.roundCount+=1
      game.rounds.unshift({id:original.id,roundNumber:original.roundNumber,ruleCode:input.ruleCode||null,note:input.note?.trim()||null,createdAt:original.createdAt,inputData:clone(input),scores:deltas.map(delta=>roundScore(game,delta)),_deltas:deltas})
      game.status=evaluation.finished?'FINISHED':'ACTIVE'
      game.outcome=evaluation.outcome??null
    }
    if(game.status==='ACTIVE'&&state.currentGame&&state.currentGame.id!==id)throw conflict('Finish or delete the current game before reopening this one.')
    game.updatedAt=new Date().toISOString()
    saveGameInSlot(state,game)
    writeState(state)
    return publicGame(game)
  },
  async undoRound(id){
    const state=readState(), game=requireGame(state,id), round=game.rounds[0]
    if(!round) throw conflict('There is no round to undo')
    for(const delta of round._deltas||[]) game.players.find(player=>player.id===delta.playerId).totalScore-=delta.points
    game.rounds.shift(); game.roundCount=Math.max(0,game.roundCount-1); game.status='ACTIVE'; game.outcome=null; game.updatedAt=new Date().toISOString()
    saveGameInSlot(state,game)
    writeState(state)
    return mutation(game,{removedRoundId:round.id})
  },
  async setStatus(id,status){
    if(!['ACTIVE','FINISHED'].includes(status)) throw bad('Unknown game status')
    const state=readState(), game=requireGame(state,id)
    if(status==='ACTIVE'&&state.currentGame&&state.currentGame.id!==id) throw conflict('Finish or delete the current game before reopening this one.')
    game.status=status; game.outcome=null; game.updatedAt=new Date().toISOString()
    if(status==='ACTIVE'){state.currentGame=game;state.historyGames=state.historyGames.filter(item=>item.id!==id)}
    else {addHistory(state,game);if(state.currentGame?.id===id)state.currentGame=null}
    writeState(state)
    return mutation(game)
  },
  exportData(){ return JSON.stringify(readState(),null,2) },
  importData(raw){
    let state
    try{state=JSON.parse(raw)}catch{throw bad('The backup file is not valid JSON')}
    const normalized=normalizeState(state);if(!normalized)throw bad('This is not a Levant Score local backup')
    writeState(normalized)
  },
}

function roundScore(game,delta){const player=game.players.find(item=>item.id===delta.playerId);if(game.scoreMode==='INDIVIDUAL')return {key:player.id,name:player.name,points:delta.points};const members=game.players.filter(item=>item.teamNumber===player.teamNumber);return {key:`team-${player.teamNumber}`,name:members.map(item=>item.name).join(' & '),points:delta.points}}
function mutation(game,extra={}){return {status:game.status,standings:calculateStandings(game),outcome:game.outcome??null,roundCount:game.roundCount,updatedAt:game.updatedAt,round:null,removedRoundId:null,...extra}}
function summary(game){const {players:_,rounds:__,...item}=publicGame(game);return item}
function publicGame(game){return {...clone(game),players:game.players.map(({totalScore:_,...player})=>player),standings:calculateStandings(game),rounds:game.rounds.slice(0,HISTORY_PAGE_SIZE).map(publicRound),hasMoreRounds:game.rounds.length>HISTORY_PAGE_SIZE}}
function publicRound(round){const {_deltas:_,...item}=round;return clone(item)}

function validateCreate(request,preset){
  ensure(request.title?.trim()&&request.title.trim().length<=120,'Game name is required and must be at most 120 characters')
  ensure(preset.allowedPlayerCounts.includes(request.players?.length),preset.allowedPlayerCounts.length===1?`${preset.name} requires exactly ${preset.allowedPlayerCounts[0]} players`:`Choose ${preset.allowedPlayerCounts.join(' or ')} players for ${preset.name}`)
  ensure(request.players.every(player=>player.name?.trim()&&player.name.trim().length<=60),'Every player needs a name of at most 60 characters')
  const teamMode=request.teamMode??preset.teamMode
  if(teamMode){ensure(request.players.every(player=>Number.isInteger(Number(player.teamNumber))&&Number(player.teamNumber)>0),'Choose a team for every player');if(preset.code!=='custom'){const counts=[...new Set(request.players.map(player=>Number(player.teamNumber)))].map(team=>request.players.filter(player=>Number(player.teamNumber)===team).length);ensure(counts.length===2&&counts[0]===counts[1],'Both teams must have the same number of players.')}}
}

function saveGameInSlot(state,game){if(game.status==='ACTIVE'){state.currentGame=game;state.historyGames=state.historyGames.filter(item=>item.id!==game.id)}else{addHistory(state,game);if(state.currentGame?.id===game.id)state.currentGame=null}}
function addHistory(state,game){state.historyGames=[game,...state.historyGames.filter(item=>item.id!==game.id)].sort((a,b)=>b.updatedAt.localeCompare(a.updatedAt)).slice(0,HISTORY_GAME_LIMIT)}
function requireGame(state,id){const game=state.currentGame?.id===id?state.currentGame:state.historyGames.find(item=>item.id===id);if(!game)throw notFound();return game}
function readState(){try{const current=normalizeState(JSON.parse(localStorage.getItem(STORAGE_KEY)||'null'));if(current)return current;const legacy=normalizeState(JSON.parse(localStorage.getItem(LEGACY_STORAGE_KEY)||'null'));return legacy||clone(EMPTY_STATE)}catch{return clone(EMPTY_STATE)}}
function normalizeState(state){if(state?.version===2&&Array.isArray(state.historyGames))return {...state,historyGames:state.historyGames.slice(0,HISTORY_GAME_LIMIT)};if(state?.version===1&&'historyGame'in state)return {version:2,currentGame:state.currentGame||null,historyGames:state.historyGame?[state.historyGame]:[]};return null}
function writeState(state){try{localStorage.setItem(STORAGE_KEY,JSON.stringify(state))}catch(error){throw new Error(error?.name==='QuotaExceededError'?'Browser storage is full. Export or delete a game and try again.':'Could not save this game on this device.')}}
function nullableNumber(value){return value==null||value===''?null:Number(value)}
function ensure(condition,message){if(!condition)throw bad(message)}
function bad(message){return new Error(message)}
function conflict(message){return new Error(message)}
function notFound(){return new Error('Game not found')}
function clone(value){return typeof structuredClone==='function'?structuredClone(value):JSON.parse(JSON.stringify(value))}
function uuid(){return globalThis.crypto?.randomUUID?.()||`${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}-${Math.random().toString(36).slice(2)}`}

