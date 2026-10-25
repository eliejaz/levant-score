export function calculateStandings(game){
  const groups=new Map()
  for(const player of game.players){
    const key=game.scoreMode==='TEAM'?`team-${player.teamNumber}`:player.id
    if(!groups.has(key))groups.set(key,[])
    groups.get(key).push(player)
  }
  const standings=[...groups].map(([key,members])=>({
    key,
    name:game.scoreMode==='TEAM'?members.map(player=>player.name).join(' & '):members[0].name,
    playerIds:members.map(player=>player.id),
    teamNumber:members[0].teamNumber,
    total:members.reduce((total,player)=>total+Number(player.totalScore||0),0),
    position:0,
  }))
  standings.sort((left,right)=>game.scoreDirection==='HIGH_WINS'?right.total-left.total:left.total-right.total)
  return standings.map((standing,index)=>({...standing,position:index+1}))
}
