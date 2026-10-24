import { api } from './api'
import { localGameRepository } from './localGameRepository'
import { getStorageMode } from './runtime'

const apiRepository={
  getPresets:()=>api('/presets'),
  listGames:({status='ACTIVE',page=1,size=6}={})=>api(`/games?status=${status}&page=${page}&size=${size}`),
  getGame:id=>api(`/games/${id}`),
  listRounds:(id,{beforeRound,limit=20}={})=>api(`/games/${id}/rounds?beforeRound=${beforeRound}&limit=${limit}`),
  createGame:request=>api('/games',{method:'POST',body:JSON.stringify(request)}),
  deleteGame:id=>api(`/games/${id}`,{method:'DELETE'}),
  rematchGame:id=>api(`/games/${id}/rematch`,{method:'POST'}),
  addRound:(id,request)=>api(`/games/${id}/rounds`,{method:'POST',body:JSON.stringify(request)}),
  undoRound:id=>api(`/games/${id}/rounds/latest`,{method:'DELETE'}),
  setStatus:(id,status)=>api(`/games/${id}/status`,{method:'PATCH',body:JSON.stringify({status})}),
}

const currentRepository=()=>getStorageMode()==='local'?localGameRepository:apiRepository
export const gameRepository=Object.fromEntries(Object.keys(apiRepository).map(method=>[method,(...args)=>currentRepository()[method](...args)]))

