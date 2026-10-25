import { useEffect, useState } from 'react'
import { ArrowLeft, ArrowRight, Check, Eye, EyeOff, Globe2 } from 'lucide-react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { useAuth } from '../lib/auth'
import { useLanguage } from '../lib/language'
import Logo from '../components/Logo'

export default function AuthPage() {
  const [params]=useSearchParams(); const navigate=useNavigate(); const initial=params.get('tab')==='signup'?'signup':'login'
  const [mode, setMode] = useState(initial); const [show, setShow] = useState(false); const [loading,setLoading]=useState(false); const [error,setError]=useState('')
  const { login, signup, chooseOnline } = useAuth(); const { lang, setLang } = useLanguage(); const ar = lang === 'ar'
  useEffect(()=>{chooseOnline()},[])
  const submit = async e => {
    e.preventDefault(); setLoading(true); setError(''); const data = Object.fromEntries(new FormData(e.currentTarget))
    try { await (mode === 'signup' ? signup(data) : login(data)); navigate('/dashboard',{replace:true}) } catch (err) { setError(authError(err,mode,ar)) } finally { setLoading(false) }
  }
  const changeMode=next=>{setMode(next);setError('')}
  return <div className="auth-page">
    <section className="auth-story">
      <div className="auth-pattern"/><Logo light />
      <div className="story-copy"><span className="eyebrow">MADE FOR THE LEVANT</span><h1>{ar ? <>كل سهرة<br/>إلها <em>حكاية.</em></> : <>Every table<br/>has a <em>story.</em></>}</h1><p>{ar ? 'نتائج دقيقة لكل ألعاب الورق اللي منحبها — من الليخة للطرنيب وأكثر.' : 'A beautiful, effortless scorekeeper for the card games we grew up with.'}</p>
        <div className="mini-score-card"><span className="card-suit">♥</span><div><small>{ar ? 'الجولة ٧' : 'ROUND 7'}</small><strong>Leekha Night</strong></div><div className="mini-points"><b>42</b><small>{ar ? 'نقطة' : 'PTS'}</small></div></div>
      </div>
      <p className="story-foot">بيروت • لبنان</p>
    </section>
    <section className="auth-form-wrap">
      <button className="auth-back" onClick={()=>navigate('/')}><ArrowLeft size={17}/>{ar?'الخيارات':'All options'}</button>
      <button className="auth-lang" onClick={()=>setLang(ar?'en':'ar')}><Globe2 size={17}/>{ar?'English':'العربية'}</button>
      <div className="auth-form-card">
        <div className="mobile-logo"><Logo /></div>
        <span className="kicker">{ar ? 'أهلاً وسهلاً' : 'AHLAN W SAHLAN'}</span>
        <h2>{mode === 'signup' ? (ar ? 'خلّينا نبلّش' : 'Let’s set the table') : (ar ? 'رجعت نورت' : 'Welcome back')}</h2>
        <p>{mode === 'signup' ? (ar ? 'حساب واحد، كل سهراتك محفوظة.' : 'One account. Every game night, remembered.') : (ar ? 'كمّل اللعبة من وين تركتها.' : 'Pick up right where your table left off.')}</p>
        <form onSubmit={submit}>
          {mode === 'signup' && <label>{ar?'اسمك':'Your name'}<input name="displayName" required maxLength="80" placeholder={ar?'مثلاً: إيلي':'e.g. Elie'}/></label>}
          <label>{ar?'البريد الإلكتروني':'Email address'}<input type="email" name="email" required placeholder="you@example.com" dir="ltr"/></label>
          <label>{ar?'كلمة السر':'Password'}<div className="password-input"><input type={show?'text':'password'} name="password" required minLength={mode==='signup'?8:1} placeholder="••••••••" dir="ltr"/><button type="button" onClick={()=>setShow(!show)}>{show?<EyeOff size={18}/>:<Eye size={18}/>}</button></div></label>
          {error && <div className="form-error">{error}</div>}
          <button className="primary-button auth-submit" disabled={loading}>{loading ? <span className="button-spinner"/> : <>{mode==='signup'?(ar?'إنشاء الحساب':'Create my account'):(ar?'دخول':'Log in')}<ArrowRight size={19}/></>}</button>
        </form>
        <div className="auth-mode-helper"><span>{mode==='login'?(ar?'ما عندك حساب؟':'Don’t have an account?'):(ar?'عندك حساب؟':'Already have an account?')}</span><button type="button" onClick={()=>changeMode(mode==='login'?'signup':'login')}>{mode==='login'?(ar?'أنشئ حساباً':'Sign up'):(ar?'سجّل الدخول':'Log in')}</button></div>
        {mode==='signup' && <div className="auth-trust"><span><Check size={14}/>{ar?'مجاني':'Free to use'}</span><span><Check size={14}/>{ar?'نتائجك محفوظة':'Scores saved'}</span></div>}
      </div>
    </section>
  </div>
}

function authError(error,mode,ar) {
  if(mode==='login'&&(error.status===401||error.status===403)) return ar?'البريد الإلكتروني أو كلمة السر غير صحيحة.':'Email or password is incorrect.'
  if(mode==='signup'&&error.status===409) return ar?'يوجد حساب مسجّل بهذا البريد الإلكتروني.':'An account already exists for this email.'
  if(error.status===400) return ar?'تحقّق من المعلومات وأعد المحاولة.':'Please check your details and try again.'
  if(error.status===0) return ar?'تعذّر الاتصال بالخادم. تحقّق من الإنترنت وحاول مجدداً.':'Could not connect to the server. Check your connection and try again.'
  return error.message&&error.message!=='Something went wrong'?error.message:(ar?'تعذّر إكمال الطلب. حاول مجدداً.':'We could not complete the request. Please try again.')
}

