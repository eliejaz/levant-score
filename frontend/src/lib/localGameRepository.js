import { gamePresets, getPreset } from './presets.js'

const STORAGE_KEY='levant-score:local:v1'
const HISTORY_PAGE_SIZE=20
const EMPTY_STATE={version:1,currentGame:null,historyGame:null}

export const localGameRepository={
  async getPresets(){ return clone(gamePresets) },
  async listGames({status='ACTIVE',page=1,size=6}={}){
    const state=readState()
    const games=[state.currentGame,state.historyGame].filter(Boolean)
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
    const preset=getPreset(request.presetCode)
    validateCreate(request,preset)
    const id=uuid(), now=new Date().toISOString()
    const teamMode=request.teamMode??preset.teamMode
    const players=request.players.map((player,index)=>({id:uuid(),name:player.name.trim(),teamNumber:teamMode?Number(player.teamNumber):null,seatOrder:index,totalScore:0}))
    const game={id,title:request.title.trim(),presetCode:preset.code,presetName:preset.code==='custom'?(request.customPresetName?.trim()||preset.name):preset.name,teamMode,scoreDirection:request.scoreDirection??preset.direction,targetScore:nullableNumber(request.targetScore??preset.defaultTarget),thresholdRule:request.thresholdRule??preset.thresholdRule,scoreMode:request.scoreMode??preset.scoreMode,roundLimit:nullableNumber(request.roundLimit??preset.roundLimit),roundCount:0,status:'ACTIVE',players,rounds:[],createdAt:now,updatedAt:now}
    state.currentGame=game
    writeState(state)
    return publicGame(game)
  },
  async deleteGame(id){
    const state=readState()
    if(state.currentGame?.id===id) state.currentGame=null
    else if(state.historyGame?.id===id) state.historyGame=null
    else throw notFound()
    writeState(state)
  },
  async rematchGame(id){
    const state=readState(), source=requireGame(state,id)
    if(state.currentGame&&state.currentGame.id!==id) throw conflict('Finish or delete the current game before starting a rematch.')
    const now=new Date().toISOString(), nextId=uuid()
    const next={...clone(source),id:nextId,status:'ACTIVE',roundCount:0,rounds:[],players:source.players.map(player=>({...player,id:uuid(),totalScore:0})),createdAt:now,updatedAt:now}
    if(source.status==='FINISHED') state.historyGame=source
    state.currentGame=next
    writeState(state)
    return publicGame(next)
  },
  async addRound(id,request){
    const state=readState(), game=requireGame(state,id)
    if(game.status==='FINISHED') throw conflict('Reopen the game before adding a round')
    if(!request.scores?.length) throw bad('scores must not be empty')
    if((request.note?.length||0)>240) throw bad('note must be at most 240 characters')
    const entries=resolveEntries(game,request)
    const points=entries.map(({input,player})=>normalizeZero(calculate(game,input,request,player)))
    validateRound(game,request,points)
    const now=new Date().toISOString(), roundId=uuid()
    const deltas=entries.map(({player},index)=>({playerId:player.id,points:points[index]}))
    for(const delta of deltas) game.players.find(player=>player.id===delta.playerId).totalScore+=delta.points
    game.roundCount+=1
    const round={id:roundId,roundNumber:game.roundCount,ruleCode:request.ruleCode||null,note:request.note?.trim()||null,createdAt:now,scores:deltas.map(delta=>roundScore(game,delta)),_deltas:deltas}
    game.rounds.unshift(round)
    const slam=game.presetCode==='400'&&request.scores.some(score=>Number(score.bid)===13)
    game.status=slam||gameEnded(game)?'FINISHED':'ACTIVE'
    game.updatedAt=now
    if(game.status==='FINISHED'){state.currentGame=null;state.historyGame=game}else state.currentGame=game
    writeState(state)
    return mutation(game,{round:publicRound(round)})
  },
  async undoRound(id){
    const state=readState(), game=requireGame(state,id), round=game.rounds[0]
    if(!round) throw conflict('There is no round to undo')
    for(const delta of round._deltas||[]) game.players.find(player=>player.id===delta.playerId).totalScore-=delta.points
    game.rounds.shift(); game.roundCount=Math.max(0,game.roundCount-1); game.updatedAt=new Date().toISOString()
    saveGameInSlot(state,game)
    writeState(state)
    return mutation(game,{removedRoundId:round.id})
  },
  async setStatus(id,status){
    if(!['ACTIVE','FINISHED'].includes(status)) throw bad('Unknown game status')
    const state=readState(), game=requireGame(state,id)
    if(status==='ACTIVE'&&state.currentGame&&state.currentGame.id!==id) throw conflict('Finish or delete the current game before reopening this one.')
    game.status=status; game.updatedAt=new Date().toISOString()
    if(status==='ACTIVE'){state.currentGame=game;if(state.historyGame?.id===id)state.historyGame=null}
    else {state.historyGame=game;if(state.currentGame?.id===id)state.currentGame=null}
    writeState(state)
    return mutation(game)
  },
  exportData(){ return JSON.stringify(readState(),null,2) },
  importData(raw){
    let state
    try{state=JSON.parse(raw)}catch{throw bad('The backup file is not valid JSON')}
    if(state?.version!==1||!('currentGame'in state)||!('historyGame'in state)) throw bad('This is not a Levant Score local backup')
    writeState(state)
  },
}

function resolveEntries(game,request){
  const used=new Set(), entries=request.scores.map(input=>{
    const player=game.scoreMode==='TEAM'&&input.teamNumber!=null?game.players.find(item=>item.teamNumber===Number(input.teamNumber)):game.players.find(item=>item.id===input.playerId)
    if(!player) throw bad(game.scoreMode==='TEAM'?'Unknown team':'Unknown player')
    if(used.has(player.id)) throw bad('Duplicate score')
    used.add(player.id); return {input,player}
  })
  const expected=game.scoreMode==='TEAM'?new Set(game.players.map(player=>player.teamNumber)).size:game.players.length
  if(used.size!==expected) throw bad('Add one score for every player or team')
  return entries
}

function calculate(game,input,request,player){
  if(game.presetCode==='400'){
    const bid=requiredNumber(input.bid,"Enter every player's bid")
    if(bid<2||bid>13) throw bad('400 bids must be between 2 and 13')
    if(bid===13)return input.made===true?41:-41
    const values={2:2,3:3,4:4,5:10,6:12,7:14,8:16,9:27,10:30,11:33,12:36}
    const value=bid>=5&&bid<=6&&player.totalScore>=30?bid:values[bid]
    return input.made===true?value:-value
  }
  if(game.presetCode==='tarneeb'){
    const bidder=requiredNumber(request.bidderTeam,'Choose the team that won the bid'), bid=requiredNumber(request.bid,'Enter the winning bid'), tricks=requiredNumber(request.tricks,"Enter the bidder's tricks"), team=requiredNumber(input.teamNumber,'Missing team')
    return team===bidder?(tricks>=bid?tricks:-bid):(tricks>=bid?0:13-tricks)
  }
  if(game.presetCode==='trex'||game.presetCode==='trex-complex'){
    if(request.ruleCode==='KING')return -75*number(input.kingHearts)
    if(request.ruleCode==='QUEENS')return -25*number(input.queens)
    if(request.ruleCode==='DIAMONDS')return -10*number(input.diamonds)
    if(request.ruleCode==='COLLECTIONS')return -15*number(input.tricks)
    if(request.ruleCode==='COMPLEX')return -75*number(input.kingHearts)-25*number(input.queens)-10*number(input.diamonds)-15*number(input.tricks)
    if(request.ruleCode==='TREX'){
      const position=requiredNumber(input.finishPosition,'Choose each finishing position')
      if(position<1||position>4)throw bad('Trex positions must be 1 to 4')
      return [200,150,100,50][position-1]
    }
    throw bad('Unknown Trex contract')
  }
  if(game.presetCode==='basra')return requiredNumber(input.cardPoints,'Enter captured card points')+requiredNumber(input.basras,'Enter the number of Basras')*10+(input.majority===true?3:0)
  if(game.presetCode==='belote')return requiredNumber(input.cardPoints,'Enter card points')+requiredNumber(input.bonusPoints,'Enter declarations and bonuses')
  if(game.presetCode==='hand')return requiredNumber(input.cardPoints,"Enter each player's cards left")
  return requiredNumber(input.points,'Enter a score')
}

function validateRound(game,request,points){
  if(game.presetCode.startsWith('leekha')){
    ensure(points.every(value=>value>=0&&value<=36),'Leekha scores must be between 0 and 36')
    ensure(sum(points)===36,'A Leekha hand must distribute exactly 36 points'); return
  }
  if(game.presetCode==='400'){
    const bids=request.scores.map(score=>requiredNumber(score.bid,"Enter every player's bid")), high=Math.max(0,...game.players.map(player=>player.totalScore)), requiredTotal=high>=50?14:high>=40?13:high>=30?12:11
    ensure(sum(bids)>=requiredTotal,`The bids must total at least ${requiredTotal} at the current table score`)
    request.scores.forEach(score=>{const player=game.players.find(item=>item.id===score.playerId), minimum=player.totalScore>=50?5:player.totalScore>=40?4:player.totalScore>=30?3:2;ensure(Number(score.bid)>=minimum,`${player.name} must bid at least ${minimum} at their current score`);ensure(typeof score.made==='boolean','Choose made or missed for every 400 bid')}); return
  }
  if(game.presetCode==='tarneeb'){
    const bidder=requiredNumber(request.bidderTeam,'Choose the bidder team'), bid=requiredNumber(request.bid,'Enter the bid'), tricks=requiredNumber(request.tricks,'Enter tricks won')
    ensure(request.scores.some(score=>Number(score.teamNumber)===bidder),'Unknown bidder team'); ensure(bid>=7&&bid<=13,'Tarneeb bids must be from 7 to 13'); ensure(tricks>=0&&tricks<=13,'Tricks must be from 0 to 13'); return
  }
  if(game.presetCode==='trex'||game.presetCode==='trex-complex'){
    const allowed=game.presetCode==='trex-complex'?['COMPLEX','TREX']:['KING','QUEENS','DIAMONDS','COLLECTIONS','TREX'], code=request.ruleCode
    ensure(allowed.includes(code),'Choose a valid contract for this Trex mode'); ensure(game.rounds.filter(round=>round.ruleCode===code).length<4,'That contract has already been played in all four kingdoms')
    const total=field=>sum(request.scores.map(score=>number(score[field])))
    if(code==='KING')ensure(total('kingHearts')===1,'Exactly one player must capture the King of Hearts')
    if(code==='QUEENS')ensure(total('queens')===4,'The four queens must all be assigned')
    if(code==='DIAMONDS')ensure(total('diamonds')===13,'The 13 diamonds must all be assigned')
    if(code==='COLLECTIONS')ensure(total('tricks')===13,'The 13 tricks must all be assigned')
    if(code==='COMPLEX'){ensure(total('kingHearts')===1,'Exactly one King of Hearts must be assigned');ensure(total('queens')===4,'The four queens must all be assigned');ensure(total('diamonds')===13,'The 13 diamonds must all be assigned');ensure(total('tricks')===13,'The 13 tricks must all be assigned')}
    if(code==='TREX')ensure(new Set(request.scores.map(score=>Number(score.finishPosition))).size===4&&request.scores.every(score=>Number(score.finishPosition)>=1&&Number(score.finishPosition)<=4),'Use each Trex finish position exactly once')
    return
  }
  if(game.presetCode==='basra'){const cards=request.scores.map(score=>requiredNumber(score.cardPoints,'Enter card points'));ensure(sum(cards)===13,'Lebanese Basra has exactly 13 special-card points per hand');ensure(request.scores.every(score=>number(score.basras)>=0&&number(score.basras)<=52),'Basra count is invalid');ensure(request.scores.filter(score=>score.majority===true).length<=1,'Only one side can receive the 3-point card majority');return}
  if(game.presetCode==='belote'){const cards=request.scores.map(score=>requiredNumber(score.cardPoints,'Enter card points'));ensure(sum(cards)===162,'Belote card points must total 162 before bonuses');ensure(request.scores.every(score=>number(score.bonusPoints)>=0&&number(score.bonusPoints)<=500),'Belote bonuses must be from 0 to 500');return}
  if(game.presetCode==='hand'){ensure(points.every(value=>value>=0&&value<=300),'Hand penalties must be between 0 and 300');ensure(points.includes(0),'At least one player must finish the hand with 0 points');return}
  ensure(points.every(value=>value>=-9999&&value<=9999),'Score is outside the supported range')
}

function gameEnded(game){
  if(game.roundLimit!=null&&game.roundCount>=game.roundLimit)return true
  if(game.targetScore==null||game.thresholdRule==='NONE'||game.roundCount===0)return false
  const current=standings(game)
  if(game.thresholdRule==='PLAYER_WIN_WITH_PARTNER_POSITIVE'){
    let threshold=game.targetScore
    while(true){const teams=new Set(current.filter(item=>item.total>=threshold&&item.teamNumber!=null&&current.filter(partner=>partner.teamNumber===item.teamNumber&&partner.key!==item.key).every(partner=>partner.total>0)).map(item=>item.teamNumber));if(teams.size===1)return true;if(teams.size<2)return false;threshold+=10}
  }
  return current.some(item=>item.total>=game.targetScore)
}

function standings(game){
  const groups=new Map()
  for(const player of game.players){const key=game.scoreMode==='TEAM'?`team-${player.teamNumber}`:player.id;if(!groups.has(key))groups.set(key,[]);groups.get(key).push(player)}
  const result=[...groups].map(([key,members])=>({key,name:game.scoreMode==='TEAM'?members.map(player=>player.name).join(' & '):members[0].name,playerIds:members.map(player=>player.id),teamNumber:members[0].teamNumber,total:sum(members.map(player=>player.totalScore)),position:0}))
  result.sort((a,b)=>game.scoreDirection==='HIGH_WINS'?b.total-a.total:a.total-b.total)
  return result.map((item,index)=>({...item,position:index+1}))
}

function roundScore(game,delta){const player=game.players.find(item=>item.id===delta.playerId);if(game.scoreMode==='INDIVIDUAL')return {key:player.id,name:player.name,points:delta.points};const members=game.players.filter(item=>item.teamNumber===player.teamNumber);return {key:`team-${player.teamNumber}`,name:members.map(item=>item.name).join(' & '),points:delta.points}}
function mutation(game,extra={}){return {status:game.status,standings:standings(game),roundCount:game.roundCount,updatedAt:game.updatedAt,round:null,removedRoundId:null,...extra}}
function summary(game){const {players:_,rounds:__,createdAt:___,...item}=publicGame(game);return item}
function publicGame(game){return {...clone(game),players:game.players.map(({totalScore:_,...player})=>player),standings:standings(game),rounds:game.rounds.slice(0,HISTORY_PAGE_SIZE).map(publicRound),hasMoreRounds:game.rounds.length>HISTORY_PAGE_SIZE}}
function publicRound(round){const {_deltas:_,...item}=round;return clone(item)}

function validateCreate(request,preset){
  ensure(request.title?.trim()&&request.title.trim().length<=120,'Game name is required and must be at most 120 characters')
  ensure(preset.allowedPlayerCounts.includes(request.players?.length),preset.allowedPlayerCounts.length===1?`${preset.name} requires exactly ${preset.allowedPlayerCounts[0]} players`:`Choose ${preset.allowedPlayerCounts.join(' or ')} players for ${preset.name}`)
  ensure(request.players.every(player=>player.name?.trim()&&player.name.trim().length<=60),'Every player needs a name of at most 60 characters')
  const teamMode=request.teamMode??preset.teamMode
  if(teamMode){ensure(request.players.every(player=>Number.isInteger(Number(player.teamNumber))&&Number(player.teamNumber)>0),'Choose a team for every player');if(preset.code!=='custom'){const counts=[...new Set(request.players.map(player=>Number(player.teamNumber)))].map(team=>request.players.filter(player=>Number(player.teamNumber)===team).length);ensure(counts.length===2&&counts[0]===counts[1],'Both teams must have the same number of players.')}}
}

function saveGameInSlot(state,game){if(game.status==='ACTIVE'){state.currentGame=game;if(state.historyGame?.id===game.id)state.historyGame=null}else{state.historyGame=game;if(state.currentGame?.id===game.id)state.currentGame=null}}
function requireGame(state,id){const game=state.currentGame?.id===id?state.currentGame:state.historyGame?.id===id?state.historyGame:null;if(!game)throw notFound();return game}
function readState(){try{const parsed=JSON.parse(localStorage.getItem(STORAGE_KEY)||'null');return parsed?.version===1?parsed:clone(EMPTY_STATE)}catch{return clone(EMPTY_STATE)}}
function writeState(state){try{localStorage.setItem(STORAGE_KEY,JSON.stringify(state))}catch(error){throw new Error(error?.name==='QuotaExceededError'?'Browser storage is full. Export or delete a game and try again.':'Could not save this game on this device.')}}
function nullableNumber(value){return value==null||value===''?null:Number(value)}
function normalizeZero(value){return Object.is(value,-0)?0:value}
function requiredNumber(value,message){if(value==null||value===''||!Number.isFinite(Number(value)))throw bad(message);return Number(value)}
function number(value){return Number(value)||0}
function sum(values){return values.reduce((total,value)=>total+Number(value),0)}
function ensure(condition,message){if(!condition)throw bad(message)}
function bad(message){return new Error(message)}
function conflict(message){return new Error(message)}
function notFound(){return new Error('Game not found')}
function clone(value){return typeof structuredClone==='function'?structuredClone(value):JSON.parse(JSON.stringify(value))}
function uuid(){return globalThis.crypto?.randomUUID?.()||`${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}-${Math.random().toString(36).slice(2)}`}

