import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { ArrowLeft, ArrowRight, Check, ChevronLeft, Plus, ShieldCheck, Users, X } from 'lucide-react'
import Shell from '../components/Shell'
import Loading from '../components/Loading'
import { api } from '../lib/api'
import { useLanguage } from '../lib/language'

export default function NewGame() {
  const [presets,setPresets]=useState(null); const [selected,setSelected]=useState(null); const [step,setStep]=useState(1); const [players,setPlayers]=useState([]); const [title,setTitle]=useState(''); const [target,setTarget]=useState(''); const [direction,setDirection]=useState('HIGH_WINS'); const [thresholdRule,setThresholdRule]=useState('NONE'); const [teamMode,setTeamMode]=useState(false); const [scoreMode,setScoreMode]=useState('INDIVIDUAL'); const [customName,setCustomName]=useState(''); const [error,setError]=useState(''); const [saving,setSaving]=useState(false)
  const {t,lang}=useLanguage(); const ar=lang==='ar'; const nav=useNavigate()
  useEffect(()=>{api('/presets').then(setPresets).catch(e=>setError(e.message))},[])
  const counts=selected?playerCounts(selected):[]; const fixedCount=counts.length===1; const discreteCount=selected?.code==='basra'
  const choose=p=>{ const allowed=playerCounts(p); const count=p.code==='basra'?2:allowed.includes(4)?4:allowed[0]; setSelected(p); setTeamMode(p.teamMode); setScoreMode(p.scoreMode); setDirection(p.direction); setThresholdRule(p.thresholdRule); setTarget(p.defaultTarget ?? ''); setTitle(ar?p.nameAr:p.name); setPlayers(Array.from({length:count},(_,i)=>({name:'',teamNumber:p.teamMode?(i%2)+1:null}))); setStep(2) }
  const setPlayer=(i,key,value)=>setPlayers(players.map((p,n)=>n===i?{...p,[key]:value}:p))
  const resizePlayers=count=>setPlayers(current=>Array.from({length:count},(_,i)=>current[i]||{name:'',teamNumber:teamMode?(i%2)+1:null}))
  const addPlayer=()=>counts.includes(players.length+1)&&resizePlayers(players.length+1)
  const removePlayer=i=>counts.includes(players.length-1)&&setPlayers(players.filter((_,n)=>n!==i))
  const toggleTeams=next=>{setTeamMode(next);setScoreMode(next?'TEAM':'INDIVIDUAL');setPlayers(players.map((p,i)=>({...p,teamNumber:next?(i%2)+1:null})))}
  const presetTeamsValid=selected?.code==='custom'||!teamMode||([1,2].every(team=>players.filter(p=>p.teamNumber===team).length===players.length/2))
  const valid=counts.includes(players.length)&&players.every(p=>p.name.trim())&&title.trim()&&(!teamMode||players.every(p=>p.teamNumber))&&presetTeamsValid
  const submit=async()=>{setSaving(true);setError('');try{const game=await api('/games',{method:'POST',body:JSON.stringify({title,presetCode:selected.code,customPresetName:customName||null,teamMode,scoreDirection:direction,targetScore:target===''?null:Number(target),thresholdRule,scoreMode,roundLimit:selected.roundLimit,players})});nav(`/games/${game.id}`)}catch(e){setError(e.message)}finally{setSaving(false)}}
  const limitLabel=thresholdRule==='LOSE_AT_OR_ABOVE'?t.losingLimit:thresholdRule==='WIN_AT_OR_ABOVE'?t.winningTarget:t.target
  return <Shell compact><div className="new-game page-container narrow">
    <button className="text-back" onClick={()=>step===2?setStep(1):nav('/')}><ChevronLeft size={18}/>{t.back}</button>
    <div className="wizard-progress"><span className="active"><b>1</b>{t.pickGame}</span><i/><span className={step===2?'active':''}><b>2</b>{t.gameDetails}</span></div>
    {step===1 ? <><div className="page-title"><span className="eyebrow green">{t.gameSetup}</span><h1>{t.pickGame}</h1><p>{ar?'اختَر اللعبة ومنظبطلك طريقة الحساب.':'Choose a favorite, or make your own rules.'}</p></div>
      {!presets?<Loading/>:<div className="preset-grid">{presets.map(p=><button key={p.code} className={`preset-card accent-${p.accent}`} onClick={()=>choose(p)}><span className="preset-suit">{presetSuit(p.code)}</span><div><h3>{ar?p.nameAr:p.name}</h3><p>{ar?p.descriptionAr:p.description}</p><span><Users size={14}/>{playerCountLabel(p,ar)} · {p.teamMode?t.teams:t.players}</span></div><ArrowRight className="preset-arrow" size={19}/></button>)}</div>}
    </> : <><div className="page-title compact-title"><span className="eyebrow green">{ar?selected.nameAr:selected.name}</span><h1>{t.gameDetails}</h1><p>{ar?'مين عم يلعب؟ فيك تغيّر كل شي قبل ما تبلّش.':'Who’s playing? You can adjust everything before you begin.'}</p></div>
      <div className="setup-layout"><section className="setup-card"><h2>{ar?'تفاصيل اللعبة':'Game details'}</h2><label>{t.gameName}<input value={title} onChange={e=>setTitle(e.target.value)} maxLength="120"/></label>{selected.code==='custom'&&<label>{ar?'نوع اللعبة':'Game type'}<input value={customName} onChange={e=>setCustomName(e.target.value)} placeholder={ar?'مثلاً: ١٤':'e.g. Fourteen'}/></label>}
        <div className="field-row"><label>{limitLabel} {thresholdRule==='NONE'&&<small>{t.optional}</small>}<input type="number" value={target} onChange={e=>setTarget(e.target.value)} placeholder="—" disabled={thresholdRule==='NONE'&&selected.code!=='custom'}/></label>{selected.code==='custom'?<label>{t.endRule}<select value={thresholdRule} onChange={e=>setThresholdRule(e.target.value)}><option value="NONE">{t.manualFinish}</option><option value="WIN_AT_OR_ABOVE">{t.firstWins}</option><option value="LOSE_AT_OR_ABOVE">{t.firstLoses}</option></select></label>:<div className="rule-field"><span>{t.endRule}</span><strong><ShieldCheck size={17}/>{ar?selected.ruleAr:selected.rule}</strong></div>}</div>
        {selected.code==='custom'&&<label>{t.scoreDirection}<select value={direction} onChange={e=>setDirection(e.target.value)}><option value="HIGH_WINS">{t.highWins}</option><option value="LOW_WINS">{t.lowWins}</option></select></label>}
        {selected.code==='custom'&&<div className="team-toggle"><div><strong>{t.teams}</strong><span>{ar?'اجمع نتائج الشركاء':'Combine partner scores'}</span></div><button className={teamMode?'on':''} onClick={()=>toggleTeams(!teamMode)}><i/></button></div>}
      </section>
      <section className="setup-card players-card"><div className="setup-card-head"><div><h2>{teamMode?t.teams:t.players}</h2><p>{fixedCount?(ar?`العدد ثابت: ${players.length}`:`Fixed at ${players.length} players`):`${players.length} ${t.players}`}</p></div>{fixedCount?<span className="fixed-player-count"><Users size={16}/>{ar?'حسب القواعد':'Game rule'}</span>:discreteCount?<div className="player-count-options" aria-label={ar?'عدد اللاعبين':'Player count'}>{counts.map(count=><button type="button" key={count} className={players.length===count?'active':''} onClick={()=>resizePlayers(count)}>{count}</button>)}</div>:<button type="button" className="small-add" onClick={addPlayer} disabled={!counts.includes(players.length+1)}><Plus size={17}/>{ar?'إضافة':'Add'}</button>}</div>
        <div className="player-count-helper">{fixedCount?(ar?`تتطلب ${selected.nameAr} أربعة لاعبين بالضبط، لذلك لا يمكن إضافة أو حذف مقعد.`:`${selected.name} requires exactly ${players.length} players, so seats cannot be added or removed.`):discreteCount?(ar?'الباصرة تُلعب بلاعبين أو بأربعة لاعبين ضمن فريقين.':'Basra supports two players or four players in two teams.'):(ar?`عدد اللاعبين المسموح: ${playerCountLabel(selected,ar)}`:`Allowed player count: ${playerCountLabel(selected,ar)}`)}</div>
        <div className="player-list">{players.map((p,i)=><div className={`player-input-row ${fixedCount||discreteCount?'locked-count':''}`} key={i}><span className={`seat-number team-${p.teamNumber||0}`}>{i+1}</span><input value={p.name} onChange={e=>setPlayer(i,'name',e.target.value)} placeholder={`${t.playerName} ${i+1}`} autoFocus={i===0}/>{teamMode&&<select value={p.teamNumber} onChange={e=>setPlayer(i,'teamNumber',Number(e.target.value))}><option value={1}>{ar?'فريق ١':'Team 1'}</option><option value={2}>{ar?'فريق ٢':'Team 2'}</option>{selected.code==='custom'&&<option value={3}>{ar?'فريق ٣':'Team 3'}</option>}</select>}{!fixedCount&&!discreteCount&&<button type="button" onClick={()=>removePlayer(i)} disabled={!counts.includes(players.length-1)} aria-label={ar?'حذف اللاعب':'Remove player'}><X size={17}/></button>}</div>)}</div>
        {!presetTeamsValid&&<div className="team-balance-error">{ar?'يجب أن يضم كل فريق العدد نفسه من اللاعبين.':'Both teams must have the same number of players.'}</div>}
      </section></div>
      {error&&<div className="form-error setup-error">{error}</div>}
      <div className="wizard-actions"><button className="secondary-button" onClick={()=>setStep(1)}><ArrowLeft size={18}/>{t.back}</button><button className="primary-button" disabled={!valid||saving} onClick={submit}>{saving?<span className="button-spinner"/>:<><Check size={19}/>{t.create}</>}</button></div>
    </>}
  </div></Shell>
}

const presetSuit=code=>code.includes('leekha')?'♥':code==='tarneeb'?'♣':code==='400'?'♦':code==='custom'?'+':'♠'
const playerCounts=p=>p.allowedPlayerCounts?.length?p.allowedPlayerCounts:Array.from({length:p.maxPlayers-p.minPlayers+1},(_,i)=>p.minPlayers+i)
const playerCountLabel=(p,ar)=>{const counts=playerCounts(p);if(counts.length===1)return `${counts[0]}`;const consecutive=counts.every((count,index)=>index===0||count===counts[index-1]+1);return consecutive?`${counts[0]}–${counts.at(-1)}`:counts.join(ar?' أو ':' or ')}
