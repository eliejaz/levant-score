import { lazy, Suspense } from 'react'
import { Navigate, Route, Routes } from 'react-router-dom'
import { useAuth } from './lib/auth'
import Loading from './components/Loading'

const AuthPage = lazy(() => import('./pages/AuthPage'))
const Dashboard = lazy(() => import('./pages/Dashboard'))
const NewGame = lazy(() => import('./pages/NewGame'))
const Scoreboard = lazy(() => import('./pages/Scoreboard'))

function Protected({ children }) { const { user } = useAuth(); return user ? children : <Navigate to="/auth" replace /> }

export default function App() {
  const { user } = useAuth()
  return <Suspense fallback={<Loading/>}><Routes>
    <Route path="/auth" element={user ? <Navigate to="/" replace /> : <AuthPage />} />
    <Route path="/" element={<Protected><Dashboard /></Protected>} />
    <Route path="/games/new" element={<Protected><NewGame /></Protected>} />
    <Route path="/games/:id" element={<Protected><Scoreboard /></Protected>} />
    <Route path="*" element={<Navigate to="/" replace />} />
  </Routes></Suspense>
}
