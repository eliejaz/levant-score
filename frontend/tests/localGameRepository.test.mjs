import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { localGameRepository as games } from '../src/lib/localGameRepository.js'

globalThis.localStorage=memoryStorage()
const rulesWasm=readFileSync(new URL('../public/game_rules.wasm',import.meta.url))
globalThis.fetch=async()=>new Response(rulesWasm,{status:200,headers:{'content-type':'application/wasm'}})

test.beforeEach(()=>localStorage.clear())

test('Leekha rejects partial hands and scores every player individually',async()=>{
  const game=await games.createGame(createRequest('leekha-team'))
  const scores=game.players.map((player,index)=>({playerId:player.id,points:[13,10,8,5][index]}))
  const result=await games.addRound(game.id,{scores})
  assert.equal(result.roundCount,1)
  assert.deepEqual(result.standings.map(item=>item.total),[5,8,10,13])
  await assert.rejects(()=>games.addRound(game.id,{scores:scores.map(score=>({...score,points:1}))}),/exactly 36 points/)
})

test('400 uses bid values and made or missed signs',async()=>{
  const game=await games.createGame(createRequest('400'))
  const bids=[
    {playerId:game.players[0].id,bid:5,made:true},
    {playerId:game.players[1].id,bid:2,made:false},
    {playerId:game.players[2].id,bid:2,made:true},
    {playerId:game.players[3].id,bid:2,made:true},
  ]
  const result=await games.addRound(game.id,{scores:bids})
  assert.deepEqual(result.standings.map(item=>item.total),[10,2,2,-2])
})

test('Tarneeb applies one team score per deal',async()=>{
  const game=await games.createGame(createRequest('tarneeb'))
  const result=await games.addRound(game.id,{bidderTeam:1,bid:8,tricks:6,scores:[{teamNumber:1},{teamNumber:2}]})
  assert.deepEqual(result.standings.map(item=>item.total),[7,-8])
  assert.deepEqual(result.round.scores.map(item=>item.points),[-8,7])
})

test('Trex, Basra, Belote, and Hand use the same preset formulas',async()=>{
  let game=await games.createGame(createRequest('trex'))
  let result=await games.addRound(game.id,{ruleCode:'KING',scores:game.players.map((player,index)=>({playerId:player.id,kingHearts:index===0?1:0}))})
  assert.deepEqual(result.round.scores.map(item=>item.points),[-75,0,0,0])
  await games.deleteGame(game.id)

  game=await games.createGame(createRequest('basra'))
  result=await games.addRound(game.id,{scores:[{teamNumber:1,cardPoints:7,basras:1,majority:true},{teamNumber:2,cardPoints:6,basras:0,majority:false}]})
  assert.deepEqual(result.standings.map(item=>item.total),[20,6])
  await games.deleteGame(game.id)

  game=await games.createGame(createRequest('belote'))
  result=await games.addRound(game.id,{scores:[{teamNumber:1,cardPoints:100,bonusPoints:20},{teamNumber:2,cardPoints:62,bonusPoints:0}]})
  assert.deepEqual(result.standings.map(item=>item.total),[120,62])
  await games.deleteGame(game.id)

  game=await games.createGame(createRequest('hand'))
  result=await games.addRound(game.id,{scores:game.players.map((player,index)=>({playerId:player.id,cardPoints:index*10}))})
  assert.deepEqual(result.standings.map(item=>item.total),[0,10,20,30])
})

test('local storage retains one active game and the latest ten finished games',async()=>{
  const first=await games.createGame(createRequest('custom'))
  await assert.rejects(()=>games.createGame(createRequest('custom')),/current game/)
  await games.setStatus(first.id,'FINISHED')
  const ids=[first.id]
  for(let index=0;index<10;index+=1){const game=await games.createGame(createRequest('custom'));ids.push(game.id);await games.setStatus(game.id,'FINISHED')}
  const page=await games.listGames({status:'FINISHED',size:20})
  assert.equal(page.totalItems,10)
  assert.equal(page.items[0].id,ids.at(-1))
  assert.ok(!page.items.some(item=>item.id===first.id))
})

test('editing a saved hand replays its scores and standings',async()=>{
  const game=await games.createGame(createRequest('leekha-team'))
  const original={scores:game.players.map((player,index)=>({playerId:player.id,points:[13,10,8,5][index]}))}
  const added=await games.addRound(game.id,original)
  await games.addRound(game.id,{scores:game.players.map(player=>({playerId:player.id,points:9}))})
  const updated=await games.updateRound(game.id,added.round.id,{note:'corrected',scores:game.players.map((player,index)=>({playerId:player.id,points:[0,12,12,12][index]}))})
  assert.deepEqual(updated.standings.map(item=>item.total),[9,21,21,21])
  assert.equal(updated.roundCount,2)
  assert.equal(updated.rounds[1].note,'corrected')
  assert.deepEqual(updated.rounds[1].inputData.scores.map(item=>item.points),[0,12,12,12])
})

function createRequest(code){
  const team=['400','tarneeb','leekha-team','basra','belote'].includes(code)
  const count=code==='custom'?2:4
  return {title:code,presetCode:code,players:Array.from({length:count},(_,index)=>({name:`P${index+1}`,teamNumber:team?(index%2)+1:null}))}
}

function memoryStorage(){
  const data=new Map()
  return {getItem:key=>data.has(key)?data.get(key):null,setItem:(key,value)=>data.set(key,String(value)),removeItem:key=>data.delete(key),clear:()=>data.clear()}
}

