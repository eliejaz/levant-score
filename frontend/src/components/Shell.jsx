import { Globe2, Home, LogOut, Plus } from 'lucide-react'
import { Link, useLocation } from 'react-router-dom'
import { useAuth } from '../lib/auth'
import { useLanguage } from '../lib/language'
import Logo from './Logo'
import { isLocalMode } from '../lib/runtime'

export default function Shell({ children, compact = false }) {
  const { user, logout } = useAuth(); const { lang, setLang, t } = useLanguage()
  const location = useLocation()
  return <div className="app-shell">
    <header className="topbar">
      <Link to="/" className="brand-link"><Logo light />{isLocalMode&&<span className="local-mode-badge">Offline</span>}</Link>
      <div className="topbar-actions">
        {!compact && <Link to="/games/new" className="top-new"><Plus size={17}/><span>{t.newGame}</span></Link>}
        <button className="language-toggle" onClick={() => setLang(lang === 'en' ? 'ar' : 'en')} aria-label="Change language"><Globe2 size={17}/>{lang === 'en' ? 'العربية' : 'English'}</button>
        <div className="avatar" title={user?.displayName}>{user?.displayName?.slice(0,1).toUpperCase()}</div>
        {!isLocalMode&&<button className="icon-button light" onClick={logout} title={t.signOut}><LogOut size={18}/></button>}
      </div>
    </header>
    <main>{children}</main>
    <nav className="mobile-nav" aria-label="Primary navigation">
      <Link to="/" className={location.pathname === '/' ? 'active' : ''}><Home/><span>{t.home}</span></Link>
      <Link to="/games/new" className={location.pathname === '/games/new' ? 'active create' : 'create'}><Plus/><span>{t.newGame}</span></Link>
      <button onClick={() => setLang(lang === 'en' ? 'ar' : 'en')}><Globe2/><span>{lang === 'en' ? 'العربية' : 'English'}</span></button>
      {!isLocalMode&&<button onClick={logout}><LogOut/><span>{t.signOut}</span></button>}
    </nav>
  </div>
}

