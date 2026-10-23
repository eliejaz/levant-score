# Implemented scoring rules

Levant Score is a scorekeeper, not a card-play simulator. It applies the scoring ownership, match length, ranking, and end condition for each preset. Players still enter the result of each hand or deal.

| Preset | Players enforced | Round entry enforced | End rule implemented |
| --- | --- | --- | --- |
| Leekha · Solo | Exactly 4 | Each player separately; exactly 36 points are distributed and every input is capped by the remaining points | First player to reach 101 loses; lowest score wins |
| Leekha · Teams | Exactly 4; two equal teams | Partners remain separate score entries; exactly 36 points per hand | If any player reaches 101, that player's team loses |
| 400 | Exactly 4; two equal teams | Every player declares a legal bid and marks made/missed; bid values and rising minimums are calculated automatically | A player reaches 41 with a positive partner; a made or missed bid of 13 ends the match immediately |
| Tarneeb | Exactly 4; two equal teams | Bidder team, contract (7–13), and tricks (0–13); both team scores are derived | First team to 31 wins; failed contracts are subtracted |
| Trex | Exactly 4 | Contract-specific totals: K♥ 1, queens 4, diamonds 13, collections 13, or unique Trex finish positions | 20 deals: four kingdoms × five contracts; highest total wins |
| Trex Complex | Exactly 4 | Complex distributes every penalty card/trick; Trex uses unique finish positions | 8 deals: four kingdoms × Complex and Trex; highest total wins |
| Belote | Exactly 4; two equal teams | Team card points must total 162 before declarations/bonuses | First team to 501 wins |
| Lebanese Basra | Exactly 2, or 4 in two equal teams | Special-card points total 13; one majority bonus at most; Basras add 10 each | First side to 101 wins |
| Hand | 2–4 individual players | At least one player is at zero; remaining-card penalties are bounded | Seven rounds; lowest total wins |
| Chinese · House rules | 2–4 individual players | Each player separately by default, with an explicit house-rule helper | Manual finish because “Chinese” refers to multiple incompatible scoring systems |
| Any game | 2–12, configurable | Configurable manual values with safe numeric bounds | User-selected scoring direction and end condition |

## Sources used

- Leekha: Qahwa and Leikha.com rule references
- 400: the documented Lebanese/Levantine 41-point rules and Jawaker reference
- Tarneeb: Tarneeb Masters rules
- Trex and Complex: documented kingdom and contract scoring references
- Lebanese Basra and Hand: Pagat regional rule references
- Belote: standard 501-point match rules

Regional and household variations remain common. The custom preset is available when a table uses a different target or scoring convention.
