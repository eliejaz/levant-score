import { Globe2, Home, LogOut, Plus } from 'lucide-react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from '../lib/auth'
import { useLanguage } from '../lib/language'
import Logo from './Logo'

export default function Shell({ children, compact = false }) {
  const {user,mode,logout,exitMode}=useAuth(); const { lang, setLang, t } = useLanguage()
  const location=useLocation(); const navigate=useNavigate(); const local=mode==='local'
  const leave=()=>{if(local)exitMode();else logout();navigate('/',{replace:true})}
  return <div className="app-shell">
    <header className="topbar">
      <Link to="/dashboard" className="brand-link"><Logo light />{local&&<span className="local-mode-badge">Offline</span>}</Link>
      <div className="topbar-actions">
        {!compact && <Link to="/games/new" className="top-new"><Plus size={17}/><span>{t.newGame}</span></Link>}
        <button className="language-toggle" onClick={() => setLang(lang === 'en' ? 'ar' : 'en')} aria-label="Change language"><Globe2 size={17}/>{lang === 'en' ? 'العربية' : 'English'}</button>
        <div className="avatar" title={user?.displayName}>{user?.displayName?.slice(0,1).toUpperCase()}</div>
        <button className="icon-button light" onClick={leave} title={local?'Exit offline mode':t.signOut}><LogOut size={18}/></button>
      </div>
    </header>
    <main>{children}</main>
    <nav className="mobile-nav" aria-label="Primary navigation">
      <Link to="/dashboard" className={location.pathname === '/dashboard' ? 'active' : ''}><Home/><span>{t.home}</span></Link>
      <Link to="/games/new" className={location.pathname === '/games/new' ? 'active create' : 'create'}><Plus/><span>{t.newGame}</span></Link>
      <button onClick={() => setLang(lang === 'en' ? 'ar' : 'en')}><Globe2/><span>{lang === 'en' ? 'العربية' : 'English'}</span></button>
      <button onClick={leave}><LogOut/><span>{local?'Exit':t.signOut}</span></button>
    </nav>
  </div>
}

