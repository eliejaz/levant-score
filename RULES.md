# Implemented scoring rules

Levant Score is a scorekeeper, not a card-play simulator. It applies the scoring ownership, match length, ranking, and end condition for each preset. Players still enter the result of each hand or deal.

| Preset | Players enforced | Round entry enforced | End rule implemented |
| --- | --- | --- | --- |
| Leekha · Solo | Exactly 4 | Each player separately; exactly 36 points are distributed and every input is capped by the remaining points | First player to reach 101 loses; lowest score wins |
| Leekha · Teams | Exactly 4; two equal teams | Partners remain separate score entries; exactly 36 points per hand | If any player reaches 101, that player's team loses |
| Leekha · Shown card (Solo or Teams) | Exactly 4; team mode still keeps scores per player | The normal 36 base points are entered. If the Leekha (Q♠) was shown, its captor receives +13 when another player showed it (26 total) or +26 when the owner captured it (39 total) | First player to reach 201 loses; in team mode that player's team loses |
| 400 | Exactly 4; two equal teams | Every player declares a legal bid and marks made/missed; bid values and rising minimums are calculated automatically | A player reaches 41 with a positive partner; a made or missed bid of 13 ends the match immediately |
| Tarneeb | Exactly 4; two equal teams | Bidder team, contract (7–13), and tricks (0–13); both team scores are derived | First team to 31 wins; failed contracts are subtracted |
| Trex | Exactly 4 | Contract-specific totals: K♥ 1, queens 4, diamonds 13, collections 13, or unique Trex finish positions | 20 deals: four kingdoms × five contracts; highest total wins |
| Trex Complex | Exactly 4 | Complex distributes every penalty card/trick; Trex uses unique finish positions | 8 deals: four kingdoms × Complex and Trex; highest total wins |
| Belote | Exactly 4; two equal teams | Team card points must total 162 before declarations/bonuses | First team to 501 wins |
| Lebanese Basra | Exactly 2, or 4 in two equal teams | Special-card points total 13; one majority bonus at most; Basras add 10 each | First side to 101 wins |
| Hand | 2–4 individual players | At least one player is at zero; remaining-card penalties are bounded | Seven rounds; lowest total wins |
| Chinese | 2–4 individual players | Exactly one player finishes first with 0 cards. Remaining cards score 1–4 ×1, 5–9 ×2, 10–12 ×3, and all 13 cards score 40 | First player to reach 100 loses; lowest score wins |
| Quatorze (14) | 2–7 individual players | Choose the player who went out; enter each opponent's remaining-card value and Joker count. A normal winner scores −30. An all-at-once winner scores −60 and every opponent's penalty is doubled. Jokers add 25 each | First player to reach 201 loses; lowest score wins |
| Any game | 2–12, configurable | Configurable manual values with safe numeric bounds | User-selected scoring direction and end condition |

## Sources used

- Leekha: Qahwa and Leikha.com rule references
- 400: the documented Lebanese/Levantine 41-point rules and Jawaker reference
- Tarneeb: Tarneeb Masters rules
- Trex and Complex: documented kingdom and contract scoring references
- Lebanese Basra and Hand: Pagat regional rule references
- Belote: standard 501-point match rules
- Quatorze: the linked Lebanese Quatorze rules describing the 51-point opening, −30/−60 winner scores, 25-point Jokers, doubled all-at-once penalties, and 201-point loss threshold

Regional and household variations remain common. The custom preset is available when a table uses a different target or scoring convention.

For this app's Chinese preset, poker combinations are used to empty the hand and the French suit order is **Pique (♠), Cœur (♥), Trèfle (♣), Carreau (♦)**. The scorekeeper does not validate the poker combinations played; it records the first player out and calculates penalties from every other player's cards left.
