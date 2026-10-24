const presets = [
  preset('leekha-solo','Leekha · Solo','ليخة · فردي','Every player keeps an individual penalty score.','كل لاعب يحافظ على نقاطه الفردية.',false,[4],'LOW_WINS',101,'LOSE_AT_OR_ABOVE','INDIVIDUAL',null,'Each player scores separately; the first to 101 loses.','كل لاعب يُحسب وحده؛ أول من يصل إلى ١٠١ يخسر.','coral'),
  preset('leekha-team','Leekha · Teams','ليخة · فرق','Partners play together but keep separate scores.','الشريكان يلعبان معاً لكن نقاط كل لاعب منفصلة.',true,[4],'LOW_WINS',101,'LOSE_AT_OR_ABOVE','INDIVIDUAL',null,'Every player scores separately. If one reaches 101, their team loses.','نقاط كل لاعب منفصلة. إذا وصل أحدهم إلى ١٠١ يخسر فريقه.','coral'),
  preset('400','400','٤٠٠','Partners play together; bids and scores stay individual.','الشريكان يلعبان معاً لكن الطلب والنقاط فردية.',true,[4],'HIGH_WINS',41,'PLAYER_WIN_WITH_PARTNER_POSITIVE','INDIVIDUAL',null,'A player reaches 41 and their partner must be above zero.','يصل لاعب إلى ٤١ ويجب أن تكون نتيجة شريكه فوق الصفر.','gold'),
  preset('tarneeb','Tarneeb','طرنيب','Two partnerships share trick and contract scores.','فريقان يتشاركان نتيجة اللمّات والطلب.',true,[4],'HIGH_WINS',31,'WIN_AT_OR_ABOVE','TEAM',null,'The first team to reach 31 wins; failed bids are subtracted.','أول فريق يصل إلى ٣١ يفوز؛ الطلب الفاشل يُطرح.','green'),
  preset('chinese','Chinese · House rules','صيني · قواعد السهرة',"The name covers several games; configure your table's scoring.",'الاسم يشمل ألعاباً مختلفة؛ اضبطوا حساب سهرتكم.',false,[2,3,4],'LOW_WINS',null,'NONE','INDIVIDUAL',null,"Chinese Poker supports 2–4 players; use your table's scoring and finish manually.",'الصيني يدعم من لاعبين إلى ٤؛ استخدموا حساب سهرتكم والإنهاء اليدوي.','blue'),
  preset('trex','Trex','تركس','Four kingdoms and five contracts per kingdom.','أربع ممالك وخمس ألعاب في كل مملكة.',false,[4],'HIGH_WINS',null,'NONE','INDIVIDUAL',20,'After 20 deals, the highest total wins.','بعد ٢٠ توزيعة يفوز صاحب أعلى مجموع.','violet'),
  preset('trex-complex','Trex Complex','تركس كومبلكس','Complex and Trex across four kingdoms.','كومبلكس وتركس عبر أربع ممالك.',false,[4],'HIGH_WINS',null,'NONE','INDIVIDUAL',8,'After 8 deals, the highest total wins.','بعد ٨ توزيعات يفوز صاحب أعلى مجموع.','violet'),
  preset('belote','Belote','بلوت','Partners share one match score.','الشريكان يتشاركان نتيجة المباراة.',true,[4],'HIGH_WINS',501,'WIN_AT_OR_ABOVE','TEAM',null,'The first team to reach 501 wins.','أول فريق يصل إلى ٥٠١ يفوز.','blue'),
  preset('basra','Lebanese Basra','باصرة لبنانية','Play head-to-head or as two partnerships.','العبوا لاعب ضد لاعب أو فريقين.',true,[2,4],'HIGH_WINS',101,'WIN_AT_OR_ABOVE','TEAM',null,'Lebanese Basra is played by 2 players or 4 players in two teams.','الباصرة اللبنانية تُلعب بلاعبين أو ٤ لاعبين ضمن فريقين.','green'),
  preset('hand','Hand','هاند','Individual penalties over seven rounds.','نقاط فردية سلبية عبر سبع جولات.',false,[2,3,4],'LOW_WINS',null,'NONE','INDIVIDUAL',7,'Hand supports 2–4 individual players; after 7 rounds the lowest total wins.','الهاند تدعم من لاعبين إلى ٤؛ بعد ٧ جولات يفوز صاحب أقل مجموع.','violet'),
  preset('custom','Any game','أي لعبة','Choose teams, scoring, and the end condition.','اختَر الفرق وطريقة الحساب وشرط النهاية.',false,range(2,12),'HIGH_WINS',null,'NONE','INDIVIDUAL',null,'Choose the scoring and end condition yourself.','اختَر طريقة الحساب وشرط النهاية بنفسك.','neutral'),
]

function preset(code,name,nameAr,description,descriptionAr,teamMode,allowedPlayerCounts,direction,defaultTarget,thresholdRule,scoreMode,roundLimit,rule,ruleAr,accent) {
  return {code,name,nameAr,description,descriptionAr,teamMode,minPlayers:allowedPlayerCounts[0],maxPlayers:allowedPlayerCounts.at(-1),allowedPlayerCounts,direction,defaultTarget,thresholdRule,scoreMode,roundLimit,rule,ruleAr,accent}
}

function range(from,to) { return Array.from({length:to-from+1},(_,index)=>from+index) }

export const gamePresets = Object.freeze(presets)
export const getPreset = code => gamePresets.find(item=>item.code===code) || gamePresets.at(-1)

