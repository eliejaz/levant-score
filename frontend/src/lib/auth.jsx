import { createContext, useContext, useEffect, useState } from 'react'
import { api } from './api'

const AuthContext = createContext(null)

export function AuthProvider({ children }) {
  const [user, setUser] = useState(() => JSON.parse(localStorage.getItem('levant-user') || 'null'))
  const save = data => {
    localStorage.setItem('levant-token', data.token); localStorage.setItem('levant-user', JSON.stringify(data.user)); setUser(data.user)
  }
  const login = async values => save(await api('/auth/login', { method: 'POST', body: JSON.stringify(values) }))
  const signup = async values => save(await api('/auth/signup', { method: 'POST', body: JSON.stringify(values) }))
  const logout = () => { localStorage.removeItem('levant-token'); localStorage.removeItem('levant-user'); setUser(null) }
  useEffect(() => { window.addEventListener('levant:unauthorized', logout); return () => window.removeEventListener('levant:unauthorized', logout) }, [])
  return <AuthContext.Provider value={{ user, login, signup, logout }}>{children}</AuthContext.Provider>
}
export const useAuth = () => useContext(AuthContext)

