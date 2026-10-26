const ENGINE_VERSION=3
const RULES_URL=`/game_rules.wasm?v=${ENGINE_VERSION}`
const encoder=new TextEncoder()
const decoder=new TextDecoder()
let enginePromise

export async function evaluateRound(game,request){
  const engine=await loadEngine()
  const input=encoder.encode(JSON.stringify({game:engineGame(game),request:engineRequest(request)}))
  const pointer=engine.alloc(input.length)
  new Uint8Array(engine.memory.buffer,pointer,input.length).set(input)
  let resultPointer
  try{resultPointer=engine.evaluate_round_json(pointer,input.length)}finally{engine.dealloc(pointer,input.length)}
  const response=JSON.parse(readResult(engine,resultPointer))
  if(!response.ok)throw new Error(response.error||'The game rules rejected this hand')
  return response.result
}

export async function getGamePresets(){
  const engine=await loadEngine()
  return JSON.parse(readResult(engine,engine.presets_json()))
}

function readResult(engine,pointer){
  const length=engine.result_len()
  const bytes=new Uint8Array(engine.memory.buffer,pointer,length).slice()
  engine.result_dealloc(pointer,length)
  return decoder.decode(bytes)
}

async function loadEngine(){
  if(!enginePromise)enginePromise=fetch(RULES_URL,{cache:'force-cache'}).then(async response=>{
    if(!response.ok)throw new Error('Could not load the game rules engine')
    let instance
    try{({instance}=await WebAssembly.instantiateStreaming(response.clone(),{}))}
    catch{({instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}))}
    if(instance.exports.engine_version()!==ENGINE_VERSION)throw new Error('The game rules engine is out of date. Refresh the page.')
    return instance.exports
  }).catch(error=>{enginePromise=null;throw error})
  return enginePromise
}

function engineGame(game){
  return {
    presetCode:game.presetCode,
    teamMode:Boolean(game.teamMode),
    scoreDirection:game.scoreDirection,
    scoreMode:game.scoreMode,
    targetScore:game.targetScore,
    thresholdRule:game.thresholdRule,
    roundLimit:game.roundLimit,
    roundCount:game.roundCount,
    players:game.players.map(player=>({id:player.id,name:player.name,teamNumber:player.teamNumber,totalScore:playerTotal(game,player)})),
    priorRoundRules:game.rounds.map(round=>round.ruleCode??null),
  }
}

function engineRequest(request){
  return {
    ruleCode:request.ruleCode??null,
    bidderTeam:nullableNumber(request.bidderTeam),
    bid:nullableNumber(request.bid),
    tricks:nullableNumber(request.tricks),
    leekhaShownByPlayerId:request.leekhaShownByPlayerId??null,
    leekhaCapturedByPlayerId:request.leekhaCapturedByPlayerId??null,
    scores:(request.scores||[]).map(score=>({
      playerId:score.playerId??null,teamNumber:nullableNumber(score.teamNumber),points:nullableNumber(score.points),
      bid:nullableNumber(score.bid),made:score.made??null,kingHearts:nullableNumber(score.kingHearts),queens:nullableNumber(score.queens),
      diamonds:nullableNumber(score.diamonds),tricks:nullableNumber(score.tricks),finishPosition:nullableNumber(score.finishPosition),
      cardPoints:nullableNumber(score.cardPoints),bonusPoints:nullableNumber(score.bonusPoints),basras:nullableNumber(score.basras),majority:score.majority??null,
      cardsLeft:nullableNumber(score.cardsLeft),
    })),
  }
}

function playerTotal(game,player){
  if(Number.isFinite(player.totalScore))return player.totalScore
  const standing=game.standings?.find(item=>item.playerIds?.includes(player.id))
  if(!standing)return 0
  if(game.scoreMode==='TEAM')return standing.playerIds?.[0]===player.id?standing.total:0
  return standing.total
}

function nullableNumber(value){return value==null||value===''?null:Number(value)}

