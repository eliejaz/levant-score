import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { ArrowUpRight, CalendarDays, ChevronLeft, ChevronRight, Plus, Spade, Trash2, X } from 'lucide-react'
import Shell from '../components/Shell'
import Loading from '../components/Loading'
import { api } from '../lib/api'
import { useAuth } from '../lib/auth'
import { useLanguage } from '../lib/language'

export default function Dashboard() {
  const [result,setResult]=useState(null); const [filter,setFilter]=useState('ACTIVE'); const [page,setPage]=useState(1); const [error,setError]=useState(''); const [reload,setReload]=useState(0)
  const [deleteTarget,setDeleteTarget]=useState(null); const [deleteError,setDeleteError]=useState(''); const [deleting,setDeleting]=useState(false)
  const { user }=useAuth(); const { t, lang }=useLanguage()
  useEffect(()=>{ let active=true; setResult(null); setError(''); api(`/games?status=${filter}&page=${page}&size=${PAGE_SIZE}`).then(data=>{if(active)setResult(normalizePage(data,filter,page))}).catch(e=>{if(active)setError(e.message)}); return()=>{active=false} },[filter,page,reload])
  const pageCount=Math.max(1,result?.totalPages||1)
  const safePage=Math.min(page,pageCount)
  const pageGames=result?.items||[]
  const visiblePages=paginationWindow(safePage,pageCount)
  useEffect(()=>{ if(result&&page>pageCount) setPage(pageCount) },[result,page,pageCount])
  const selectFilter=next=>{ setFilter(next); setPage(1) }
  const selectPage=next=>{
    setPage(Math.max(1,Math.min(next,pageCount)))
    document.querySelector('.games-section')?.scrollIntoView({behavior:'smooth',block:'start'})
  }
  const requestDelete=game=>{ setDeleteTarget(game); setDeleteError('') }
  const cancelDelete=()=>{ if(!deleting){ setDeleteTarget(null); setDeleteError('') } }
  const deleteGame=async()=>{
    if(!deleteTarget) return
    setDeleting(true); setDeleteError('')
    try {
      await api(`/games/${deleteTarget.id}`,{method:'DELETE'})
      setDeleteTarget(null)
      if(pageGames.length===1&&page>1) setPage(current=>current-1); else setReload(current=>current+1)
    } catch(e) { setDeleteError(e.message) }
    finally { setDeleting(false) }
  }
  return <Shell><div className="dashboard page-container">
    <section className="welcome-row"><div><span className="eyebrow green">{t.welcome}</span><h1>{t.tableReady}, {user.displayName.split(' ')[0]}.</h1><p>{lang==='ar'?'كمّل لعبة أو جهّز سهرة جديدة.':'Continue a game or set up a new card night.'}</p></div><Link className="primary-button" to="/games/new"><Plus size={19}/>{t.newGame}</Link></section>
    <section className="dashboard-stats">
      <div className="stat-feature"><div className="stat-icon">♠</div><div><small>{lang==='ar'?'كل الجولات':'ALL-TIME ROUNDS'}</small><strong>{result?.stats.allTimeRounds||0}</strong></div><span>{lang==='ar'?'اللعب حلو':'Keep the good hands coming'}</span></div>
      <div className="stat-simple"><small>{t.activeGames}</small><strong>{result?.stats.activeGames||0}</strong><span className="live-dot">● {t.live}</span></div>
      <div className="stat-simple"><small>{t.finished}</small><strong>{result?.stats.finishedGames||0}</strong><CalendarDays size={20}/></div>
    </section>
    <section className="games-section"><div className="section-head"><div><span className="eyebrow green">{lang==='ar'?'سهرتك':'YOUR TABLES'}</span><h2>{lang==='ar'?'الألعاب الأخيرة':'Recent games'}</h2></div><div className="segmented"><button className={filter==='ACTIVE'?'active':''} onClick={()=>selectFilter('ACTIVE')}>{t.activeGames}</button><button className={filter==='FINISHED'?'active':''} onClick={()=>selectFilter('FINISHED')}>{t.finished}</button></div></div>
      {result===null ? error ? <div className="empty-state">{error}</div> : <Loading/> : pageGames.length===0 ? <div className="empty-state"><div className="empty-icon"><Spade/></div><h3>{t.noGames}</h3><p>{t.noGamesSub}</p><Link className="secondary-button" to="/games/new">{t.startGame}<ArrowUpRight size={18}/></Link></div> : <><div className="game-grid">{pageGames.map(game=><GameCard key={game.id} game={game} t={t} lang={lang} onDelete={requestDelete}/>)}</div>{pageCount>1&&<nav className="game-pagination" aria-label={lang==='ar'?'صفحات الألعاب':'Game pages'}><button className="page-direction" type="button" onClick={()=>selectPage(safePage-1)} disabled={safePage===1} aria-label={lang==='ar'?'الصفحة السابقة':'Previous page'}><ChevronLeft size={18}/><span>{lang==='ar'?'السابق':'Previous'}</span></button><div className="page-numbers">{visiblePages.map(item=>typeof item==='number'?<button key={item} type="button" className={item===safePage?'current':''} aria-current={item===safePage?'page':undefined} aria-label={`${lang==='ar'?'صفحة':'Page'} ${item}`} onClick={()=>selectPage(item)}>{item}</button>:<span key={item} className="page-ellipsis" aria-hidden="true">…</span>)}</div><button className="page-direction" type="button" onClick={()=>selectPage(safePage+1)} disabled={safePage===pageCount} aria-label={lang==='ar'?'الصفحة التالية':'Next page'}><span>{lang==='ar'?'التالي':'Next'}</span><ChevronRight size={18}/></button></nav>}</>}
    </section>
    {deleteTarget&&<div className="modal-backdrop" onMouseDown={event=>{ if(event.target===event.currentTarget) cancelDelete() }}><section className="confirm-modal" role="dialog" aria-modal="true" aria-labelledby="dashboard-delete-title"><div className="confirm-danger-icon"><Trash2 size={24}/></div><h2 id="dashboard-delete-title">{t.deleteGame}</h2><p>{t.deleteGameMessage}</p>{deleteError&&<div className="form-error">{deleteError}</div>}<div className="confirm-actions"><button type="button" className="secondary-button" onClick={cancelDelete} disabled={deleting}>{t.cancel}</button><button type="button" className="danger-button" onClick={deleteGame} disabled={deleting}><Trash2 size={17}/>{deleting?'…':t.deleteForever}</button></div></section></div>}
  </div></Shell>
}

function GameCard({game,t,lang,onDelete}) {
  const leader=game.standings?.[0]
  return <div className="game-card-wrap"><Link className="game-card" to={`/games/${game.id}`}><div className={`game-accent accent-${accent(game.presetCode)}`}><span>{suit(game.presetCode)}</span></div><div className="game-card-body"><div className="game-card-top"><span className={`status-pill ${game.status.toLowerCase()}`}>{game.status==='ACTIVE'?t.live:t.finished}</span><span className="round-count">{game.roundCount}{game.roundLimit?` / ${game.roundLimit}`:''} {t.rounds}</span></div><h3>{game.title}</h3><p>{game.presetName} · {game.teamMode?t.teams:t.players}</p>{leader && <div className="leader-row"><div><small>{game.status==='FINISHED'?t.winner:t.leading}</small><strong>{leader.name}</strong></div><b>{leader.total}<small>{t.points}</small></b></div>}<ChevronRight className="card-arrow" size={21}/></div></Link><button type="button" className="game-card-delete" onClick={()=>onDelete(game)} aria-label={`${t.deleteGame}: ${game.title}`} title={t.deleteGame}><X size={18}/></button></div>
}
const accent=code=>code.includes('leekha')?'coral':code==='400'?'gold':code==='tarneeb'?'green':code==='chinese'?'blue':'violet'
const suit=code=>code.includes('leekha')?'♥':code==='tarneeb'?'♣':code==='400'?'♦':'♠'
const PAGE_SIZE=6
function normalizePage(data,status,page){
  if(!Array.isArray(data)) return data
  const filtered=data.filter(game=>game.status===status); const start=(page-1)*PAGE_SIZE
  return {items:filtered.slice(start,start+PAGE_SIZE),page,size:PAGE_SIZE,totalItems:filtered.length,totalPages:Math.ceil(filtered.length/PAGE_SIZE),stats:{allTimeRounds:data.reduce((sum,game)=>sum+game.roundCount,0),activeGames:data.filter(game=>game.status==='ACTIVE').length,finishedGames:data.filter(game=>game.status==='FINISHED').length}}
}
function paginationWindow(current,total){
  if(total<=5) return Array.from({length:total},(_,index)=>index+1)
  const pages=[1]
  if(current>3) pages.push('left-gap')
  for(let value=Math.max(2,current-1);value<=Math.min(total-1,current+1);value++) pages.push(value)
  if(current<total-2) pages.push('right-gap')
  pages.push(total)
  return pages
}
