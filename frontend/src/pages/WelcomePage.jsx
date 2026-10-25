import { ArrowRight, Cloud, Globe2, LogIn, ShieldCheck, UserPlus, WifiOff } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import Logo from '../components/Logo'
import { useAuth } from '../lib/auth'
import { useLanguage } from '../lib/language'

export default function WelcomePage(){
  const {accountUser,chooseOnline,chooseOffline}=useAuth(); const {lang,setLang}=useLanguage(); const navigate=useNavigate(); const ar=lang==='ar'
  const online=tab=>{chooseOnline();navigate(`/auth?tab=${tab}`)}
  const offline=()=>{chooseOffline();navigate('/dashboard')}
  return <main className="welcome-page">
    <header className="welcome-header"><Logo light/><button onClick={()=>setLang(ar?'en':'ar')}><Globe2 size={17}/>{ar?'English':'العربية'}</button></header>
    <section className="welcome-copy"><span className="eyebrow">{ar?'عداد ورق لبناني وعربي':'LEBANESE & ARAB CARD SCOREKEEPER'}</span><h1>{ar?'كيف بدّك تبلّش؟':'How do you want to play?'}</h1><p>{ar?'اختَر حساباً لحفظ كل سهراتك، أو العب فوراً بلا إنترنت ومن دون تسجيل.':'Save every card night with an account, or start instantly on this device without signing in.'}</p></section>
    <section className="welcome-options" aria-label={ar?'خيارات الدخول':'Choose how to continue'}>
      <button className="welcome-option signup" onClick={()=>online('signup')}><span className="welcome-option-icon"><UserPlus/></span><span className="welcome-option-copy"><small>{ar?'جديد هون؟':'NEW HERE'}</small><strong>{ar?'إنشاء حساب':'Sign up'}</strong><em>{ar?'احفظ ألعابك وافتحها من أي جهاز.':'Keep your games and open them from any device.'}</em></span><ArrowRight className="welcome-arrow"/></button>
      <button className="welcome-option login" onClick={()=>online('login')}><span className="welcome-option-icon"><LogIn/></span><span className="welcome-option-copy"><small>{ar?'عندك حساب؟':'HAVE AN ACCOUNT'}</small><strong>{ar?'تسجيل الدخول':'Log in'}</strong><em>{accountUser?(ar?`الحساب المحفوظ: ${accountUser.displayName}`:`Saved account: ${accountUser.displayName}`):(ar?'كمّل ألعابك المحفوظة.':'Continue your saved games.')}</em></span><ArrowRight className="welcome-arrow"/></button>
      <button className="welcome-option offline" onClick={offline}><span className="welcome-option-icon"><WifiOff/></span><span className="welcome-option-copy"><small>{ar?'بلا حساب':'NO ACCOUNT'}</small><strong>{ar?'اللعب بلا إنترنت':'Play offline'}</strong><em>{ar?'لعبة حالية وآخر ١٠ ألعاب في السجل على هذا الجهاز.':'One current game and your latest 10 games on this device.'}</em></span><ArrowRight className="welcome-arrow"/></button>
    </section>
    <footer className="welcome-notes"><span><Cloud size={16}/>{ar?'الحساب: مزامنة وحفظ':'Account: synced and saved'}</span><span><ShieldCheck size={16}/>{ar?'بلا إنترنت: بياناتك تبقى على جهازك':'Offline: data stays on your device'}</span></footer>
  </main>
}
