import test from 'node:test'
import assert from 'node:assert/strict'
import { localGameRepository as games } from '../src/lib/localGameRepository.js'

globalThis.localStorage=memoryStorage()

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

test('local storage retains at most one active game and one finished game',async()=>{
  const first=await games.createGame(createRequest('custom'))
  await assert.rejects(()=>games.createGame(createRequest('custom')),/current game/)
  await games.setStatus(first.id,'FINISHED')
  const second=await games.createGame(createRequest('custom'))
  let page=await games.listGames({status:'FINISHED'})
  assert.equal(page.totalItems,1)
  await games.setStatus(second.id,'FINISHED')
  page=await games.listGames({status:'FINISHED'})
  assert.equal(page.totalItems,1)
  assert.equal(page.items[0].id,second.id)
})

function createRequest(code){
  const team=code==='400'||code==='tarneeb'||code==='leekha-team'
  const count=code==='custom'?2:4
  return {title:code,presetCode:code,players:Array.from({length:count},(_,index)=>({name:`P${index+1}`,teamNumber:team?(index%2)+1:null}))}
}

function memoryStorage(){
  const data=new Map()
  return {getItem:key=>data.has(key)?data.get(key):null,setItem:(key,value)=>data.set(key,String(value)),removeItem:key=>data.delete(key),clear:()=>data.clear()}
}

