import { createContext, useContext, useEffect, useState } from 'react'
import { api } from './api'
import { isLocalMode } from './runtime'

const AuthContext = createContext(null)
const LOCAL_USER={id:'local-device',displayName:'Offline player',email:null}

export function AuthProvider({ children }) {
  const [user, setUser] = useState(() => isLocalMode ? LOCAL_USER : JSON.parse(localStorage.getItem('levant-user') || 'null'))
  const save = data => {
    localStorage.setItem('levant-token', data.token); localStorage.setItem('levant-user', JSON.stringify(data.user)); setUser(data.user)
  }
  const login = async values => save(await api('/auth/login', { method: 'POST', body: JSON.stringify(values) }))
  const signup = async values => save(await api('/auth/signup', { method: 'POST', body: JSON.stringify(values) }))
  const logout = () => { if(isLocalMode)return; localStorage.removeItem('levant-token'); localStorage.removeItem('levant-user'); setUser(null) }
  useEffect(() => { if(isLocalMode)return; window.addEventListener('levant:unauthorized', logout); return () => window.removeEventListener('levant:unauthorized', logout) }, [])
  return <AuthContext.Provider value={{ user, login, signup, logout }}>{children}</AuthContext.Provider>
}
export const useAuth = () => useContext(AuthContext)

