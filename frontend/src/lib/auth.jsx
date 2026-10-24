import { createContext, useContext, useEffect, useState } from 'react'
import { api, warmApi } from './api'
import { getStorageMode, setStorageMode } from './runtime'

const AuthContext = createContext(null)
const LOCAL_USER={id:'local-device',displayName:'Offline player',email:null}

export function AuthProvider({ children }) {
  const [mode,setMode]=useState(getStorageMode)
  const [accountUser,setAccountUser]=useState(readAccountUser)
  const user=mode==='local'?LOCAL_USER:mode==='api'?accountUser:null
  const save = data => {
    localStorage.setItem('levant-token', data.token); localStorage.setItem('levant-user', JSON.stringify(data.user)); setStorageMode('api'); setMode('api'); setAccountUser(data.user)
  }
  const login = async values => save(await api('/auth/login', { method: 'POST', body: JSON.stringify(values) }))
  const signup = async values => save(await api('/auth/signup', { method: 'POST', body: JSON.stringify(values) }))
  const chooseOnline=()=>{setStorageMode('api');setMode('api');warmApi()}
  const chooseOffline=()=>{setStorageMode('local');setMode('local')}
  const exitMode=()=>{setStorageMode(null);setMode(null)}
  const logout=()=>{localStorage.removeItem('levant-token');localStorage.removeItem('levant-user');setAccountUser(null);exitMode()}
  useEffect(()=>{if(mode!=='api')return;window.addEventListener('levant:unauthorized',logout);return()=>window.removeEventListener('levant:unauthorized',logout)},[mode])
  return <AuthContext.Provider value={{user,accountUser,mode,login,signup,logout,chooseOnline,chooseOffline,exitMode}}>{children}</AuthContext.Provider>
}
export const useAuth = () => useContext(AuthContext)

function readAccountUser(){try{return JSON.parse(localStorage.getItem('levant-user')||'null')}catch{return null}}

