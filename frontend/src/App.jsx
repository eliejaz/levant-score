import { Navigate, Route, Routes } from 'react-router-dom'
import { useAuth } from './lib/auth'
import AuthPage from './pages/AuthPage'
import Dashboard from './pages/Dashboard'
import NewGame from './pages/NewGame'
import Scoreboard from './pages/Scoreboard'

function Protected({ children }) { const { user } = useAuth(); return user ? children : <Navigate to="/auth" replace /> }

export default function App() {
  const { user } = useAuth()
  return <Routes>
    <Route path="/auth" element={user ? <Navigate to="/" replace /> : <AuthPage />} />
    <Route path="/" element={<Protected><Dashboard /></Protected>} />
    <Route path="/games/new" element={<Protected><NewGame /></Protected>} />
    <Route path="/games/:id" element={<Protected><Scoreboard /></Protected>} />
    <Route path="*" element={<Navigate to="/" replace />} />
  </Routes>
}
