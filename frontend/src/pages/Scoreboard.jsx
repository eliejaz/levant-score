import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { ArrowLeft, Check, ChevronDown, ChevronUp, Crown, Flag, History, Minus, Pencil, Plus, RefreshCw, RotateCcw, Trash2, Trophy, X } from 'lucide-react'
import Shell from '../components/Shell'
import Loading from '../components/Loading'
import { gameRepository } from '../lib/gameRepository'
import { evaluateRound } from '../lib/rulesEngine'
import { useLanguage } from '../lib/language'

const HISTORY_PAGE_SIZE=20
const normalizeGame=data=>({...data,roundCount:data.roundCount??data.rounds?.length??0,hasMoreRounds:data.hasMoreRounds??false})

export default function Scoreboard() {
  const {id}=useParams(); const nav=useNavigate(); const {t,lang}=useLanguage(); const ar=lang==='ar'
  const [game,setGame]=useState(null); const [error,setError]=useState(''); const [modal,setModal]=useState(false); const [editingRound,setEditingRound]=useState(null); const [deleteOpen,setDeleteOpen]=useState(false); const [busy,setBusy]=useState(false); const [loadingMore,setLoadingMore]=useState(false); const [expanded,setExpanded]=useState(null)
  useEffect(()=>{
    let active=true
    setGame(null); setError('')
    gameRepository.getGame(id).then(data=>{if(active)setGame(normalizeGame(data))}).catch(e=>{if(active)setError(e.message)})
    return ()=>{active=false}
  },[id])
  const applyMutation=(mutation,kind)=>setGame(current=>{
    if(mutation.id&&mutation.players&&mutation.rounds) return normalizeGame(mutation)
    let rounds=current.rounds
    if(kind==='add'&&mutation.round) rounds=[mutation.round,...rounds].slice(0,HISTORY_PAGE_SIZE)
    if(kind==='undo'&&mutation.removedRoundId) rounds=rounds.filter(round=>round.id!==mutation.removedRoundId)
    return {...current,status:mutation.status,standings:mutation.standings,roundCount:mutation.roundCount,updatedAt:mutation.updatedAt,rounds,hasMoreRounds:mutation.roundCount>rounds.length}
  })
  const action=async(fn,kind)=>{setBusy(true);setError('');try{const mutation=await fn();applyMutation(mutation,kind);return true}catch(e){setError(e.message);return false}finally{setBusy(false)}}
  const openAddRound=()=>{setEditingRound(null);setModal(true)}
  const openEditRound=round=>{setEditingRound(round);setModal(true)}
  const closeRoundModal=()=>{if(!busy){setModal(false);setEditingRound(null)}}
  const undo=()=>action(()=>gameRepository.undoRound(id),'undo')
  const toggleStatus=()=>action(()=>gameRepository.setStatus(id,game.status==='ACTIVE'?'FINISHED':'ACTIVE'))
  const loadMore=async()=>{const before=game.rounds.at(-1)?.roundNumber;if(!before)return;setLoadingMore(true);setError('');try{const page=await gameRepository.listRounds(id,{beforeRound:before,limit:HISTORY_PAGE_SIZE});setGame(current=>({...current,rounds:[...current.rounds,...page.items.filter(item=>!current.rounds.some(round=>round.id===item.id))],hasMoreRounds:page.hasMore}))}catch(e){setError(e.message)}finally{setLoadingMore(false)}}
  const rematch=async()=>{setBusy(true);setError('');try{const next=await gameRepository.rematchGame(id);nav(`/games/${next.id}`)}catch(e){setError(e.message)}finally{setBusy(false)}}
  const deleteGame=async()=>{setBusy(true);setError('');try{await gameRepository.deleteGame(id);nav('/dashboard',{replace:true})}catch(e){setError(e.message);setDeleteOpen(false)}finally{setBusy(false)}}
  if(!game)return <Shell compact><div className="page-container"><button className="text-back" onClick={()=>nav('/dashboard')}><ArrowLeft size={18}/>{t.back}</button>{error?<div className="empty-state">{error}</div>:<Loading/>}</div></Shell>

  const leader=game.standings[0]; const hasRounds=game.roundCount>0
  const thresholdLoser=game.thresholdRule==='LOSE_AT_OR_ABOVE'&&game.targetScore!=null
    ? game.standings.filter(s=>s.total>=game.targetScore).sort((a,b)=>b.total-a.total)[0] : null
  const winningThresholdReached=(game.thresholdRule==='WIN_AT_OR_ABOVE'||game.thresholdRule==='PLAYER_WIN_WITH_PARTNER_POSITIVE')&&game.targetScore!=null&&game.standings.some(s=>s.total>=game.targetScore)
  const roundsFinished=game.roundLimit!=null&&game.roundCount>=game.roundLimit
  const slamScore=game.presetCode==='400'&&game.status==='FINISHED'?game.rounds[0]?.scores.find(s=>Math.abs(s.points)===41):null
  const slamPlayer=slamScore?game.players.find(p=>p.id===slamScore.key):null
  const slamWinningTeam=slamPlayer?(slamScore.points>0?slamPlayer.teamNumber:game.players.find(p=>p.teamNumber!==slamPlayer.teamNumber)?.teamNumber):null
  const automaticResult=game.status==='FINISHED'&&(thresholdLoser||winningThresholdReached||roundsFinished||slamScore)
  const winnerName=slamWinningTeam
    ? teamName(game,slamWinningTeam)
    : thresholdLoser&&game.teamMode
    ? teamName(game,game.players.find(p=>p.teamNumber!==thresholdLoser.teamNumber)?.teamNumber)
    : game.teamMode&&game.scoreMode==='INDIVIDUAL'&&leader?.teamNumber
      ? teamName(game,leader.teamNumber) : leader?.name
  const dangerScore=thresholdLoser?.total??game.standings[game.standings.length-1]?.total??0

  return <Shell compact><div className="scoreboard">
    <section className="score-hero"><div className="page-container"><button className="hero-back" onClick={()=>nav('/dashboard')}><ArrowLeft size={18}/>{t.home}</button><div className="score-title-row"><div><div className="game-meta"><span className={`status-pill ${game.status.toLowerCase()}`}>{game.status==='ACTIVE'?t.live:t.finished}</span><span>{game.presetName}</span><span>•</span><span>{game.roundCount}{game.roundLimit?` / ${game.roundLimit}`:''} {t.rounds}</span></div><h1>{game.title}</h1></div><div className="score-actions">{game.status==='FINISHED'&&<button className="hero-secondary rematch-button" onClick={rematch} disabled={busy}><RefreshCw size={17}/>{t.playAgain}</button>}{hasRounds&&game.status==='ACTIVE'&&<button className="hero-secondary" onClick={undo} disabled={busy}><RotateCcw size={17}/>{t.undo}</button>}<button className="hero-secondary" onClick={toggleStatus} disabled={busy}><Flag size={17}/>{game.status==='ACTIVE'?t.finish:t.reopen}</button><button className="hero-secondary delete-game-button" onClick={()=>setDeleteOpen(true)} disabled={busy} title={t.deleteGame} aria-label={t.deleteGame}><Trash2 size={17}/><span>{t.deleteGame}</span></button></div></div>
      {leader&&hasRounds&&(automaticResult
        ? <div className="leader-banner game-over-banner"><div className="leader-crown"><Trophy size={24}/></div><div><small>{t.gameOver}</small><strong>{winnerName} {t.wins}</strong></div>{slamScore?<div className="result-reason">{slamScore.name} {slamScore.points>0?(ar?'نجح بطلب ١٣':'made bid 13'):(ar?'خسر طلب ١٣':'missed bid 13')}</div>:thresholdLoser&&<div className="result-reason">{thresholdLoser.name} {t.reached} {game.targetScore}</div>}{roundsFinished&&!slamScore&&<div className="result-reason">{game.roundCount} / {game.roundLimit} {t.rounds}</div>}</div>
        : <div className="leader-banner"><div className="leader-crown"><Crown size={24}/></div><div><small>{t.leading}</small><strong>{leader.name}</strong></div><div className="leader-score"><b>{leader.total}</b><span>{t.points}</span></div>{game.targetScore!=null&&<div className="target-progress"><span>{game.thresholdRule==='LOSE_AT_OR_ABOVE'?t.losingLimit:t.targetLabel} {game.targetScore}</span><div><i style={{width:`${Math.min(100,Math.abs((game.thresholdRule==='LOSE_AT_OR_ABOVE'?dangerScore:leader.total)/game.targetScore*100))}%`}}/></div></div>}</div>)}
      </div></section>
    <div className="page-container score-content"><section className="standings-section"><div className="section-label"><span className="eyebrow green">{ar?'الترتيب':'STANDINGS'}</span><p>{game.scoreDirection==='HIGH_WINS'?t.highWins:t.lowWins}</p></div><div className="standings-grid">{game.standings.map((s,i)=><div className={`standing-card ${i===0&&hasRounds?'first':''} ${thresholdLoser?.key===s.key?'limit-reached':''}`} key={s.key}><div className="rank">{hasRounds&&i===0?<Trophy size={18}/>:i+1}</div><div className="standing-avatar">{s.name.slice(0,1).toUpperCase()}</div><div className="standing-name"><strong>{s.name}</strong>{game.teamMode&&<small>{ar?`فريق ${s.teamNumber}`:`Team ${s.teamNumber}`}</small>}</div><div className="standing-total"><b>{s.total}</b><small>{t.points}</small></div></div>)}</div>
      {game.status==='ACTIVE'&&<button className="add-round-main" onClick={openAddRound}><span><Plus size={25}/></span><div><strong>{t.addRound}</strong><small>{game.scoreMode==='INDIVIDUAL'?(ar?'سجّل نتيجة كل لاعب':'Enter a score for every player'):(ar?'سجّل نتيجة كل فريق':'Enter a score for each team')}</small></div></button>}</section>
      <section className="history-section"><div className="history-head"><div><span className="eyebrow green">{t.history}</span><h2>{ar?'كل توزيعة، محفوظة.':'Every hand, remembered.'}</h2></div><History size={25}/></div>{!hasRounds?<div className="no-rounds"><span>♠</span><p>{t.noRounds}</p></div>:<><div className="round-list">{game.rounds.map(r=><div className={`round-row ${expanded===r.id?'expanded':''}`} key={r.id}><button onClick={()=>setExpanded(expanded===r.id?null:r.id)}><span className="round-number">{r.roundNumber}</span><div className="round-copy"><strong>{t.round} {r.roundNumber}</strong><small>{r.note||new Date(r.createdAt).toLocaleTimeString(lang==='ar'?'ar-LB':'en-LB',{hour:'2-digit',minute:'2-digit'})}</small></div><div className="round-preview">{r.scores.map(s=><span key={s.key}><i>{initials(s.name)}</i><b className={s.points<0?'negative':''}>{s.points>0?'+':''}{s.points}</b></span>)}</div>{expanded===r.id?<ChevronUp size={18}/>:<ChevronDown size={18}/>}</button>{expanded===r.id&&<div className="round-details">{r.scores.map(s=><div key={s.key}><span>{s.name}</span><b className={s.points<0?'negative':''}>{s.points>0?'+':''}{s.points}</b></div>)}{r.inputData?<button type="button" className="edit-round-button" onClick={()=>openEditRound(r)}><Pencil size={15}/>{ar?'تعديل التوزيعة':'Edit hand'}</button>:<small className="legacy-round-note">{ar?'يمكن تعديل التوزيعات الجديدة فقط.':'Only newly saved hands can be edited.'}</small>}</div>}</div>)}</div>{game.hasMoreRounds&&<button type="button" className="secondary-button history-load-more" onClick={loadMore} disabled={loadingMore}>{loadingMore?'…':ar?'تحميل توزيعات أقدم':'Load older hands'}</button>}</>}</section>
      {error&&<div className="form-error floating-error">{error}</div>}
    </div>
    {game.status==='ACTIVE'&&<button className="mobile-add-score" onClick={openAddRound}><Plus size={23}/><span>{t.addRound}</span></button>}
    {modal&&<RoundModal game={game} round={editingRound} t={t} ar={ar} onClose={closeRoundModal} onSave={async body=>{const saved=editingRound?await action(()=>gameRepository.updateRound(id,editingRound.id,body),'edit'):await action(()=>gameRepository.addRound(id,body),'add');if(saved){setModal(false);setEditingRound(null)}}} busy={busy}/>}
    {deleteOpen&&<div className="modal-backdrop" onMouseDown={e=>!busy&&e.target===e.currentTarget&&setDeleteOpen(false)}><section className="confirm-modal" role="dialog" aria-modal="true" aria-labelledby="delete-game-title"><div className="confirm-danger-icon"><Trash2 size={24}/></div><h2 id="delete-game-title">{t.deleteGame}</h2><p>{t.deleteGameMessage}</p><div className="confirm-actions"><button type="button" className="secondary-button" onClick={()=>setDeleteOpen(false)} disabled={busy}>{t.cancel}</button><button type="button" className="danger-button" onClick={deleteGame} disabled={busy}>{busy?<span className="button-spinner"/>:<><Trash2 size={18}/>{t.deleteForever}</>}</button></div></section></div>}
  </div></Shell>
}

function RoundModal({game,round,t,ar,onClose,onSave,busy}) {
  const initialRequest=round?.inputData
  const scoringGame=useMemo(()=>round?gameBeforeRound(game,round):game,[game,round])
  const participants=game.scoreMode==='TEAM'
    ? game.standings.slice().sort((a,b)=>(a.teamNumber||0)-(b.teamNumber||0))
    : game.players.map(p=>({key:p.id,name:p.name,playerIds:[p.id],teamNumber:p.teamNumber}))
  const initialEntry=p=>({points:0,bid:min400Bid(scoringGame,p),made:true,kingHearts:0,queens:0,diamonds:0,tricks:0,finishPosition:participants.indexOf(p)+1,cardPoints:0,bonusPoints:0,basras:0,majority:false})
  const savedEntry=p=>initialRequest?.scores?.find(score=>game.scoreMode==='TEAM'?Number(score.teamNumber)===Number(p.teamNumber):score.playerId===p.playerIds[0])
  const [entries,setEntries]=useState(Object.fromEntries(participants.map(p=>[p.key,{...initialEntry(p),...savedEntry(p)}])))
  const [note,setNote]=useState(initialRequest?.note||''); const [ruleCode,setRuleCode]=useState(initialRequest?.ruleCode||(game.presetCode==='trex-complex'?'COMPLEX':'KING'))
  const [bidderTeam,setBidderTeam]=useState(initialRequest?.bidderTeam??participants[0]?.teamNumber??1); const [contractBid,setContractBid]=useState(initialRequest?.bid??7); const [contractTricks,setContractTricks]=useState(initialRequest?.tricks??7)
  const [validation,setValidation]=useState({ok:false,error:null})
  const field=(key,name,value)=>setEntries({...entries,[key]:{...entries[key],[name]:value===''?'':Number(value)}})
  const toggle=(key,name,value)=>setEntries({...entries,[key]:{...entries[key],[name]:value}})
  const scoreInputs=participants.map(p=>({
    ...(game.scoreMode==='TEAM'?{teamNumber:p.teamNumber}:{playerId:p.playerIds[0]}),
    ...entries[p.key], points:Number(entries[p.key].points||0)
  }))
  const roundRequest={note,ruleCode,bidderTeam,bid:Number(contractBid),tricks:Number(contractTricks),scores:scoreInputs}
  const requestKey=JSON.stringify(roundRequest)
  useEffect(()=>{let active=true;setValidation({ok:false,error:null});evaluateRound(scoringGame,roundRequest).then(()=>{if(active)setValidation({ok:true,error:null})}).catch(error=>{if(active)setValidation({ok:false,error:error.message})});return()=>{active=false}},[scoringGame,requestKey])
  const submit=e=>{e.preventDefault();if(validation.ok)onSave(roundRequest)}
  const helper=roundHelper(scoringGame,participants,entries,{ruleCode,bidderTeam,contractBid,contractTricks},ar)
  return <div className="modal-backdrop" onMouseDown={e=>e.target===e.currentTarget&&onClose()}><div className="round-modal"><div className="modal-head"><div><span className="eyebrow green">{t.round} {round?.roundNumber??game.roundCount+1}{game.roundLimit?` / ${game.roundLimit}`:''}</span><h2>{round?(ar?'تعديل التوزيعة':'Edit hand'):roundTitle(game,ar)}</h2></div><button onClick={onClose}><X size={22}/></button></div><form onSubmit={submit}>
    <div className={`rule-helper ${validation.ok?'ready':'waiting'}`}><strong>{helper.title}</strong><span>{helper.text}</span>{validation.error&&<small>{validation.error}</small>}</div>
    {(game.presetCode==='trex'||game.presetCode==='trex-complex')&&<label className="contract-picker">{ar?'العقد':'Contract'}<select value={ruleCode} onChange={e=>setRuleCode(e.target.value)}>{trexContracts(game.presetCode,ar).map(c=><option key={c.code} value={c.code} disabled={game.rounds.filter(r=>r.id!==round?.id&&r.ruleCode===c.code).length>=4}>{c.label}</option>)}</select></label>}
    {game.presetCode==='tarneeb'&&<div className="contract-grid"><label>{ar?'الفريق الطالب':'Bidder team'}<select value={bidderTeam} onChange={e=>setBidderTeam(Number(e.target.value))}>{participants.map(p=><option key={p.key} value={p.teamNumber}>{p.name}</option>)}</select></label><label>{ar?'الطلب':'Bid'}<input type="number" min="7" max="13" value={contractBid} onChange={e=>setContractBid(Number(e.target.value))}/></label><label>{ar?'الأكلات المأخوذة':'Tricks won'}<input type="number" min="0" max="13" value={contractTricks} onChange={e=>setContractTricks(Number(e.target.value))}/></label></div>}
    <div className="score-inputs">{participants.map((p,i)=><RuleEntry key={p.key} game={scoringGame} participant={p} index={i} entry={entries[p.key]} entries={entries} ruleCode={ruleCode} ar={ar} field={field} toggle={toggle} bidderTeam={bidderTeam} contractBid={contractBid} contractTricks={contractTricks}/>)}</div>
    <label className="note-field">{t.note}<input value={note} onChange={e=>setNote(e.target.value)} maxLength="240" placeholder={t.noteHint}/></label><div className="modal-actions"><button type="button" className="secondary-button" onClick={onClose}>{ar?'إلغاء':'Cancel'}</button><button className="primary-button" disabled={busy||!validation.ok}>{busy?<span className="button-spinner"/>:<><Check size={18}/>{round?(ar?'حفظ التعديل':'Save changes'):t.saveRound}</>}</button></div>
  </form></div></div>
}

function RuleEntry({game,participant:p,index,entry,entries,ruleCode,ar,field,toggle,bidderTeam,contractBid,contractTricks}) {
  const id=p.key; const preset=game.presetCode
  let controls
  if(preset==='400') controls=<><label className="compact-input">{ar?'الطلب':'Bid'}<input type="number" min={min400Bid(game,p)} max="13" value={entry.bid} onChange={e=>field(id,'bid',e.target.value)}/></label><div className="made-toggle"><button type="button" className={entry.made?'active':''} onClick={()=>toggle(id,'made',true)}>✓ {ar?'أخذ':'Made'}</button><button type="button" className={!entry.made?'missed':''} onClick={()=>toggle(id,'made',false)}>− {ar?'كسر':'Missed'}</button></div><b className={`score-preview ${entry.made?'':'negative'}`}>{entry.made?'+':'−'}{fourHundredValue(entry.bid,standingTotal(game,p))}</b></>
  else if(preset==='tarneeb') { const score=p.teamNumber===Number(bidderTeam)?(contractTricks>=contractBid?contractTricks:-contractBid):(contractTricks>=contractBid?0:13-contractTricks); controls=<b className={`score-preview ${score<0?'negative':''}`}>{score>0?'+':''}{score}</b> }
  else if(preset==='trex'||preset==='trex-complex') controls=<TrexControls id={id} entry={entry} ruleCode={ruleCode} ar={ar} field={field}/>
  else if(preset==='basra') controls=<><label className="compact-input">{ar?'نقاط الورق':'Card pts'}<input type="number" min="0" max="13" value={entry.cardPoints} onChange={e=>field(id,'cardPoints',e.target.value)}/></label><label className="compact-input">{ar?'باصرة':'Basras'}<input type="number" min="0" value={entry.basras} onChange={e=>field(id,'basras',e.target.value)}/></label><button type="button" className={`majority-button ${entry.majority?'active':''}`} onClick={()=>toggle(id,'majority',!entry.majority)}>+3 {ar?'أكثر ورق':'Most cards'}</button></>
  else if(preset==='belote') controls=<><label className="compact-input">{ar?'نقاط الورق':'Card pts'}<input type="number" min="0" max="162" value={entry.cardPoints} onChange={e=>field(id,'cardPoints',e.target.value)}/></label><label className="compact-input">{ar?'إعلانات':'Bonuses'}<input type="number" min="0" max="500" value={entry.bonusPoints} onChange={e=>field(id,'bonusPoints',e.target.value)}/></label></>
  else if(preset==='hand') controls=<NumberControl value={entry.cardPoints} min={0} max={300} onChange={v=>field(id,'cardPoints',v)}/>
  else if(preset.startsWith('leekha')) { const available=Math.max(0,36-Object.entries(entries).filter(([key])=>key!==id).reduce((sum,[,value])=>sum+Number(value.points||0),0)); controls=<NumberControl value={entry.points} min={0} max={available} onChange={v=>field(id,'points',v===''?'':Math.max(0,Math.min(available,Number(v))))}/> }
  else controls=<NumberControl value={entry.points} min={-9999} max={9999} onChange={v=>field(id,'points',v)}/>
  return <div className="score-input-row rule-entry"><span className={`score-player-dot dot-${index}`}>{initials(p.name)}</span><div className="rule-player"><strong>{p.name}</strong>{game.teamMode&&<small>{ar?'فريق':'Team'} {p.teamNumber}</small>}</div><div className="rule-controls">{controls}</div></div>
}

function NumberControl({value,min,max,onChange}) { const n=Number(value||0); return <div className="number-stepper"><button type="button" disabled={n<=min} onClick={()=>onChange(n-1)}><Minus size={18}/></button><input type="number" min={min} max={max} value={value} onFocus={()=>{if(n===0)onChange('')}} onBlur={()=>{if(value==='')onChange(0)}} onChange={e=>onChange(e.target.value)}/><button type="button" disabled={n>=max} onClick={()=>onChange(n+1)}><Plus size={18}/></button></div> }

function TrexControls({id,entry,ruleCode,ar,field}) {
  if(ruleCode==='TREX') return <label className="compact-input finish-input">{ar?'الترتيب':'Finish'}<select value={entry.finishPosition} onChange={e=>field(id,'finishPosition',e.target.value)}>{[1,2,3,4].map(n=><option key={n} value={n}>#{n} · {[200,150,100,50][n-1]}</option>)}</select></label>
  const controls=[]
  if(ruleCode==='KING'||ruleCode==='COMPLEX') controls.push(<label className="compact-input" key="k">K♥<input type="number" min="0" max="1" value={entry.kingHearts} onChange={e=>field(id,'kingHearts',e.target.value)}/></label>)
  if(ruleCode==='QUEENS'||ruleCode==='COMPLEX') controls.push(<label className="compact-input" key="q">{ar?'بنات':'Queens'}<input type="number" min="0" max="4" value={entry.queens} onChange={e=>field(id,'queens',e.target.value)}/></label>)
  if(ruleCode==='DIAMONDS'||ruleCode==='COMPLEX') controls.push(<label className="compact-input" key="d">♦<input type="number" min="0" max="13" value={entry.diamonds} onChange={e=>field(id,'diamonds',e.target.value)}/></label>)
  if(ruleCode==='COLLECTIONS'||ruleCode==='COMPLEX') controls.push(<label className="compact-input" key="c">{ar?'أكلات':'Tricks'}<input type="number" min="0" max="13" value={entry.tricks} onChange={e=>field(id,'tricks',e.target.value)}/></label>)
  return controls
}

function roundHelper(game,participants,entries,state,ar) {
  const values=participants.map(p=>entries[p.key]); const sum=name=>values.reduce((n,v)=>n+Number(v[name]||0),0); const p=game.presetCode
  if(p.startsWith('leekha')) { const left=36-sum('points'); return {title:ar?'٣٦ نقطة في كل توزيعة':'36 points in every hand',text:left===0?(ar?'تم توزيع كل النقاط — جاهز للحفظ.':'All 36 points are assigned — ready to save.'):(ar?`باقي ${left} نقطة للتوزيع.`:`${left} points still need to be assigned.`)} }
  if(p==='400') { const high=Math.max(...game.standings.map(s=>s.total),0); const needed=high>=50?14:high>=40?13:high>=30?12:11; const slam=values.some(v=>Number(v.bid)===13); return {title:ar?'كل لاعب يطلب ويُحاسب وحده':'Every player bids and scores separately',text:slam?(ar?'طلب ١٣: النجاح يفوز بالمباراة فوراً والفشل يخسرها فوراً.':'Bid 13: making it wins the match immediately; missing it loses immediately.'):(ar?`مجموع الطلب ${sum('bid')} من ${needed} كحد أدنى. النجاح يضيف قيمة الطلب والفشل يطرحها.`:`Total bids: ${sum('bid')} / minimum ${needed}. Made adds the bid value; missed subtracts it.`)} }
  if(p==='tarneeb') return {title:ar?'١٣ أكلة في التوزيعة':'13 tricks in every deal',text:state.contractTricks>=state.contractBid?(ar?'الفريق الطالب نجح؛ تُضاف الأكلات التي أخذها.':'Contract made: the bidder scores its tricks.'):(ar?'الطلب مكسور؛ يُطرح الطلب والخصم يأخذ أكلاته.':'Contract failed: subtract the bid; defenders score their tricks.')}
  if(p==='trex'||p==='trex-complex') { const targets={KING:['K♥',1,'kingHearts'],QUEENS:[ar?'البنات':'Queens',4,'queens'],DIAMONDS:[ar?'الديناري':'Diamonds',13,'diamonds'],COLLECTIONS:[ar?'الأكلات':'Tricks',13,'tricks']}; if(state.ruleCode==='TREX')return {title:ar?'ترتيب فريد من الأول للرابع':'Unique finish order, first through fourth',text:ar?'النقاط: ٢٠٠، ١٥٠، ١٠٠، ٥٠.':'Points: 200, 150, 100, 50.'}; if(state.ruleCode==='COMPLEX')return {title:'Complex: K♥ + Queens + Diamonds + Tricks',text:`K♥ ${sum('kingHearts')}/1 · Q ${sum('queens')}/4 · ♦ ${sum('diamonds')}/13 · ${ar?'أكلات':'tricks'} ${sum('tricks')}/13`}; const [name,target,key]=targets[state.ruleCode];return {title:`${name}: ${sum(key)} / ${target}`,text:ar?'وزّع كل الورق أو الأكلات قبل الحفظ.':'Assign every card or trick before saving.'} }
  if(p==='basra') return {title:ar?'١٣ نقطة ورق + ٣ للأكثر + ١٠ لكل باصرة':'13 card points + 3 majority + 10 per Basra',text:ar?`نقاط الورق: ${sum('cardPoints')} / 13`:`Card points assigned: ${sum('cardPoints')} / 13`}
  if(p==='belote') return {title:ar?'١٦٢ نقطة ورق في كل توزيعة':'162 card points in every deal',text:ar?`الموزّع: ${sum('cardPoints')} / 162. أضف الإعلانات منفصلة.`:`Assigned: ${sum('cardPoints')} / 162. Add declarations separately.`}
  if(p==='hand') return {title:ar?'الفائز يسجّل صفر':'The player who goes out scores zero',text:ar?'أدخل قيمة الورق المتبقي لكل لاعب.':'Enter the value of the cards left in each hand.'}
  return {title:ar?'قواعد السهرة':'House-rule scoring',text:ar?'هذه اللعبة غير موحّدة؛ أدخلوا فقط القيم المتفق عليها قبل البدء.':'This variant has no single documented score system; use the values agreed before play.'}
}

const roundTitle=(game,ar)=>game.presetCode==='400'?(ar?'سجّل الطلب والنتيجة':'Record bids and results'):game.presetCode==='tarneeb'?(ar?'سجّل الطلب والأكلات':'Record contract and tricks'):game.presetCode.startsWith('trex')?(ar?'سجّل عقد تركس':'Score a Trex contract'):ar?'أضف توزيعة':'Add hand'
function gameBeforeRound(game,round){
  const base=structuredClone(game)
  for(const player of base.players){
    const standing=base.standings.find(item=>item.playerIds?.includes(player.id))
    player.totalScore=base.scoreMode==='TEAM'&&standing?.playerIds?.[0]!==player.id?0:standing?.total||0
  }
  for(const savedRound of base.rounds.filter(item=>item.roundNumber>=round.roundNumber)){
    for(const score of savedRound.scores){
      const player=base.scoreMode==='TEAM'
        ? base.players.find(item=>`team-${item.teamNumber}`===score.key)
        : base.players.find(item=>item.id===score.key)
      if(player)player.totalScore-=score.points
    }
  }
  base.roundCount=round.roundNumber-1
  base.rounds=base.rounds.filter(item=>item.roundNumber<round.roundNumber)
  base.status='ACTIVE'
  base.standings=standingsFromPlayers(base)
  return base
}
function standingsFromPlayers(game){
  const groups=new Map()
  for(const player of game.players){const key=game.scoreMode==='TEAM'?`team-${player.teamNumber}`:player.id;if(!groups.has(key))groups.set(key,[]);groups.get(key).push(player)}
  return [...groups].map(([key,members])=>({key,name:members.map(item=>item.name).join(' & '),playerIds:members.map(item=>item.id),teamNumber:members[0].teamNumber,total:members.reduce((sum,item)=>sum+item.totalScore,0)})).sort((a,b)=>game.scoreDirection==='HIGH_WINS'?b.total-a.total:a.total-b.total)
}
const standingTotal=(game,p)=>{const player=game.players.find(item=>item.id===p.playerIds[0]);return Number.isFinite(player?.totalScore)?player.totalScore:game.standings.find(s=>s.playerIds?.includes(p.playerIds[0]))?.total||0}
const min400Bid=(game,p)=>{const total=standingTotal(game,p);return total>=50?5:total>=40?4:total>=30?3:2}
const fourHundredValue=(bid,total)=>Number(bid)===13?41:Number(bid)>=5&&Number(bid)<=6&&total>=30?Number(bid):({2:2,3:3,4:4,5:10,6:12,7:14,8:16,9:27,10:30,11:33,12:36}[Number(bid)]||0)
const trexContracts=(preset,ar)=>preset==='trex-complex'?[{code:'COMPLEX',label:ar?'كومبلكس':'Complex'},{code:'TREX',label:ar?'تركس':'Trex'}]:[{code:'KING',label:ar?'ملك القلوب':'King of Hearts'},{code:'QUEENS',label:ar?'البنات':'Queens'},{code:'DIAMONDS',label:ar?'الديناري':'Diamonds'},{code:'COLLECTIONS',label:ar?'الأكلات':'Collections'},{code:'TREX',label:ar?'تركس':'Trex'}]

const teamName=(game,teamNumber)=>game.players.filter(p=>p.teamNumber===teamNumber).map(p=>p.name).join(' & ')
const initials=name=>name.split(/\s|&/).filter(Boolean).slice(0,2).map(x=>x[0]).join('').toUpperCase()
