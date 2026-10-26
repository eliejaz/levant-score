export function calculateRoundTotals(game){
  const running=new Map((game.standings||[]).map(standing=>[standing.key,Number(standing.total)||0]))
  const totals={}
  const rounds=[...(game.rounds||[])].sort((a,b)=>b.roundNumber-a.roundNumber)
  for(const round of rounds){
    totals[round.id]=Object.fromEntries((round.scores||[]).map(score=>[score.key,running.get(score.key)??0]))
    for(const score of round.scores||[])running.set(score.key,(running.get(score.key)??0)-Number(score.points||0))
  }
  return totals
}
