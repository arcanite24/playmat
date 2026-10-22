# Career difficulty calibration

Goal: a new player with a starter deck should beat the Apprentice tier (1) most of the time, the Journeyman
tier (2) about half the time, and the Champion tier (3) less often, roughly 70% / 50% / 30%.

## Method

Humans can't be measured in a test, so the AI stands in for the player.
`Mage.Server/src/test/java/mage/server/career/CareerCalibrationTest.java` plays whole headless games:

- **Player side:** each 60-card starter deck from `starters/index.json`, played by the server AI
  (`ComputerPlayerControllableProxy`, the "Computer - mad" player) at a fixed **reference skill 4**.
- **Opponent side:** each opponent from `opponents.json`, with its own deck and skill, seated the same way
  `CareerApi` seats it: same AI class, and the opponent's `skill` override or else the tier's skill.
- The starters take turns against each opponent. The starter side goes first in half the games.
- Mulligans and opening hands are real. A game still running at turn 30 (both players' turns counted) is a
  draw, and a draw counts as half a win. A game the engine ends on an internal error is left out of the numbers.
- **Score** is the starter side's win rate per opponent and per tier.

The test is skipped unless `-Dxmage.careerCalibration=true` is set.

## Rerun

From the repository root (this needs a card database, which the first run builds in `Mage.Server/db`):

```sh
mvn -o -pl Mage.Server.Plugins/Mage.Player.AI.MAD,Mage.Server test -Dtest=CareerCalibrationTest \
    -Dsurefire.failIfNoSpecifiedTests=false -Dxmage.careerCalibration=true \
    -Dxmage.careerCalibration.games=36 \
    -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties
```

Surefire captures the output, so read the table (and a `RESULT,...` line per game) in
`Mage.Server/target/surefire-reports/TEST-mage.server.career.CareerCalibrationTest.xml`. An average game takes about 5 seconds, so 432 games
(36 per opponent) take about 35 minutes on one JVM.

Options (system properties, all prefixed `xmage.careerCalibration.`):

| property | default | meaning |
|---|---|---|
| `games` | 12 | games per opponent; the starters take turns, so 12 means one game per starter |
| `referenceSkill` | 4 | skill of the AI playing the starters |
| `maxTurns` | 30 | turn at which a game is called a draw |
| `opponents` | all | comma-separated opponent ids, e.g. `wren,pip` |
| `tierSkills` | from json | try other tier skills without editing the json, e.g. `1,4,7` |
| `shard` | `0/1` | `i/n` plays every n-th game starting at game i, to split a run across JVMs |

To run in parallel, start several JVMs with `shard=0/4` ... `shard=3/4`. The `main` method runs the
calibration directly, so this doesn't need Maven. Start the JVMs about 15 seconds apart, because they share
the H2 card database and opening it at the same moment can fail. Each game prints a `RESULT,...` line. To merge
the logs, pass them as arguments to `main`:
`java -cp <test classpath> mage.server.career.CareerCalibrationTest shard0.log shard1.log ...`

## Findings (2026-10-09)

**Before**, with tiers at skills 1/4/7 and the original decks (24 games per opponent):

| tier | skill | starter score |
|---|---|---|
| 1 Apprentice | 1 | 51% |
| 2 Journeyman | 4 | 55% |
| 3 Champion | 7 | 32% |

Skill 1 played as well as skill 4. Below 4, every skill searched 4 actions deep, and the only difference was
think time, which these board states never use up. Tier 3 also had a weak deck: Mirela's `br-vampires`
(Decks to Beat, Br Vampires, March 2011) scored 59 to 62% for the starters.

**Changes:**

1. **AI (`ComputerPlayer6.rootScoreNoiseForSkill`):** skills 1, 2 and 3 now misjudge their own options. Each
   first action they can take gets a random error of up to ±2500 (skill 1), ±1500 (skill 2) or ±700 (skill 3)
   score points, and then the best-scoring action is picked. For scale, a creature on the battlefield is worth
   roughly 1000 points. Won or lost games are never blurred, and skill 4 and above search exactly as before.
   Measured for tier 1 (starter score): ±600 gave 49%, ±1500 gave 62%, ±2500 gave 65% (192 games) and ±3000
   gave 67%. Skills 2 and 3 are interpolated; Career doesn't use them.
2. **Deck:** Mirela now plays `rakdos-aristocrats.dck`, a copy of
   `Mage.Client/release/sample-decks/Decks to Beat/BR Zombies TDtB ST Dec 2012.mwDeck.dck` (Blood Artist,
   Falkenrath Aristocrat, Gravecrawler). It comes from the same folder and source as the other tier-3 decks.
   The candidates scored as follows at skill 7, 24 games each:

   | candidate | starter score |
   |---|---|
   | Rakdos Aggro, December 2012 | 8% (too hard) |
   | BR Zombies, December 2012 (chosen) | 25% |
   | BR Zombies, August 2012 | 33% |
   | Rakdos Aggro, November 2012 | 42% |
   | Rakdos Aggro, January 2013 | 44% |
   | Br Vampires: three other versions | 62 to 67% |

3. Tier skills stay at 1/4/7.

**After** (36 games per opponent, 432 games, 0 engine errors):

| opponent | tier | skill | W-L-D | starter score |
|---|---|---|---|---|
| wren | 1 | 1 | 30-5-1 | 85% |
| odo | 1 | 1 | 25-10-1 | 71% |
| pip | 1 | 1 | 20-14-2 | 58% |
| marisol | 1 | 1 | 19-13-4 | 58% |
| hask | 2 | 4 | 21-15-0 | 58% |
| widow | 2 | 4 | 18-15-3 | 54% |
| ilse | 2 | 4 | 21-9-6 | 67% |
| grondel | 2 | 4 | 15-20-1 | 43% |
| sera | 3 | 7 | 9-27-0 | 25% |
| korvath | 3 | 7 | 13-23-0 | 36% |
| mirela | 3 | 7 | 7-29-0 | 19% |
| aldous | 3 | 7 | 10-26-0 | 28% |
| **Tier 1** | | | 94-42-8 | **68%** |
| **Tier 2** | | | 75-59-10 | **56%** |
| **Tier 3** | | | 39-105-0 | **27%** |

With 36 games, one opponent's score has a standard error of about 8 points; for a whole tier (144 games) it's
about 4 points.

## AI search budgets (2026-10-12)

The AI's node budget now follows its skill: 2,000 nodes at skills 1–2, 3,500 at skill 3, and 5,000 (as before) above.
A search also no longer starts when the player has nothing but mana abilities to play. Only tier 1 (skill 1)
plays with a smaller budget, so it was rerun (24 games per opponent, 96 games, 0 engine errors):

| opponent | tier | skill | W-L-D | starter score |
|---|---|---|---|---|
| wren | 1 | 1 | 15-7-2 | 67% |
| odo | 1 | 1 | 18-5-1 | 77% |
| pip | 1 | 1 | 14-10-0 | 58% |
| marisol | 1 | 1 | 15-9-0 | 63% |
| **Tier 1** | | | 62-31-3 | **66%** (68% before) |

That is within the run's standard error (about 5 points), so the tiers stand. The 96 games took 88 seconds,
under a second a game.

## The schools (2026-10-09)

The academy's one-on-one duels were measured with the schools mode: each node's loaner played by the reference AI
(skill 4) against the node's opponent at its own skill, 4 games per node (12 for the nodes changed below). Commander
and Brawl duels and the act 4 pods were left out then; see "Commander and pods" below.

```sh
java -cp <test classpath> -Dxmage.careerCalibration.schools=pauper,limited -Dxmage.careerCalibration.games=4 \
    -Dxmage.careerCalibration.shard=0/4 mage.server.career.CareerCalibrationTest
```

`schools` takes school ids (or `all`), `nodes` limits the run to node ids, and the merged table's "tier" is the act.

Starter-side score per act (games in brackets):

| school | act 1 | act 2 | act 3 | act 4 |
|---|---|---|---|---|
| pauper | 75% (12) | 62% (16) | 25% (8) | 28% (16) |
| limited | 58% (12) | 53% (16) | 62% (8) | 47% (16) |
| pioneer | 75% (12) | 38% (16) | 25% (8) | 3% (16) |
| standard | 58% (12) | 38% (16) | 50% (8) | 44% (16) |
| modern | 55% (28) | 12% (16) | 0% (8) | 6% (16) |
| legacy | 48% (20) | 16% (16) | 50% (8) | 44% (16) |
| vintage | 75% (12) | 9% (16) | 38% (8) | 22% (16) |
| **all** | **61%** (108) | **33%** (112) | **36%** (56) | **28%** (112) |

**Changes:** the first run had Modern's act 1 at 0% and 25% and Legacy's first duel at 25%: both schools opened on a
full Merfolk deck with twelve lords. The act 1 Merfolk decks (`modern/act1-merfolk`, `legacy/opp-merfolk`) now run
Coral Merfolk and Merfolk Looter instead of Master of the Pearl Trident and Merrow Reejerey, and Modern's act 1 Burn
runs Shock and Raging Goblin instead of Skewer the Critics and Searing Blaze. 12 games each after the change:
modern-1a 0% to 75%, modern-1b 25% to 42%, legacy-1a 25% to 62%.

**Reading the rest:** with 4 games a node, one node's score is noise (a standard error of about 25 points); an act's
16 games narrow it to about 12. Acts 2 to 4 hand the player combo and control loaners on purpose (CURRICULUM.md), and
the AI pilots those badly, so the starter side's score there understates a human. Still, the hot spots to watch in
play are Modern's acts 2 to 4, Pioneer's act 4, and every school's act 2 combo and control duels (`-2c`, `-2d`):
if humans lose them as often, soften the opponent's deck the way act 1 was.

## The ladder past Champion (2026-10-13)

The roster grew (S037–S048): Master and Archmage tiers (4 and 5, skills 8 and 9, with life twists), the rival's four
chapters, a raid boss on 100 life, alternate lists for every opponent, and personalities. The calibration now plays
them the way a duel seats them: list `i` in game `i`, the tier's and the opponent's starting life and life bonus
(timed twists such as a boss's phases are not applied), and the opponent's personality. Options added:
`opponentAi=mcts` seats the Monte Carlo AI, `personalities=false` drops the personalities.

**Personalities (S042)** read `ai.personality.<playerId>` (aggression, risk, patience, each -100 to 100) in
`ComputerPlayer6.applyPersonality`: aggression sends attackers that would only trade, risk sends ones that could die
for nothing, patience keeps the smallest back while the defender can hit back. Tier 2 with and without them, 48 games
each: 40% and 44% for the starters, inside the noise (about 7 points), so they change how an opponent plays more than
how often it wins. The alternate lists did move tier 2: 44% here against 56% with the single lists.

**First pass over the new opponents** (8 games each):

| opponent | tier | skill | starter score |
|---|---|---|---|
| rival-1 | 1 | 1 | 63% |
| rival-2 | 2 | 4 | 25% |
| rival-3 | 3 | 7 | 0% |
| rival-4 | 4 | 8 | 63% |
| vesna | 4 | 8 | 0% |
| thaddeus | 4 | 8 | 19% |
| orla | 4 | 8 | 50% |
| null-engine | 5 | 9 | 25% |
| saint-adela | 5 | 9 | 25% |
| leviathan | 5 | 9 | 0% |

**Changes (S043):** the school and meta decks are much stronger than the starters, so skill barely moved the hot
spots (rival-2 at skill 3 still scored 17%, then 8% on Pioneer Rakdos midrange). What worked was the deck:

- rival-2 plays the Pauper Rakdos list of chapter 1 at the tier's skill (the rival comes back sharper, not richer);
  rival-3 drops to skill 5; rival-4 gets 4 more life (it scored like a tier-2 opponent);
- vesna drops to skill 6;
- the Leviathan keeps its 100 life and phases but pilots the tier-1 Wilds and the Deep at skill 3.

After (12 games each):

| opponent | tier | skill | starter score |
|---|---|---|---|
| rival-2 | 2 | 4 | 42% |
| rival-3 | 3 | 5 | 25% |
| rival-4 | 4 | 8 | 42% |
| vesna | 4 | 6 | 17% |
| leviathan (raid) | 5 | 3 | 17% (2 draws) |

Tier 4 now sits at about 30% for the starters and the raid boss can be beaten. These are 12-game samples, a standard
error of about 14 points per opponent, so treat them as a first cut.

**Skill 9.** Tier 5's skill-9 opponents were no harder than tier 3's skill 7 (25% against 0 to 25%), and their games
ran faster, often under a second. Why isn't measured yet; until it is, tier 5's difficulty comes from its twists, and
the Archmage seat (S050) is where a stronger AI goes.

## The Monte Carlo AI (S044, 2026-10-13)

The Monte Carlo AI (`Computer - monte carlo`) couldn't play at all: its simulation threads (`AI-SIM-MCTS`) failed the
engine's game-thread check on every simulated priority (1,440 exceptions before the first game ended).
`ThreadUtils.isRunGameThread` now accepts them. Measured in the opponent's seat at skill 4 against tier 2's starters:
the starters won all 5 games played (against ilse, 11 to 17 turns), and each game took 7 to 12 minutes against
about 5 seconds with the standard AI.

It isn't seated for any opponent: it plays weaker than the standard AI at the same skill and takes several minutes a
game. `opponentAi=mcts` keeps it measurable; an opponent can still name it with `"aiType": "COMPUTER_MONTE_CARLO"`.

## Commander and pods (S043, 2026-10-13)

The schools mode now plays Commander and Brawl games with a command zone (the Commander and Brawl game types, their own
starting life) and seats every opponent of a pod; the starter side wins only by being the last one standing. A first
pass, 2 games per node:

| school act | games | starter score |
|---|---|---|
| commander act 1 | 6 | 67% |
| commander act 2 | 8 | 63% |
| commander act 3 | 4 | 0% |
| commander act 4 (pod) | 1 | 0% |

The run was stopped there (Commander games take about a minute each, so the full set of 52 is an hour); Brawl wasn't
reached. Commander acts 1 and 2 land near the curriculum's targets; act 3 is the one to rerun with more games. Run it
with `-Dxmage.careerCalibration.schools=commander,brawl`.

## Caveats

- The AI is not a human. A human who plays better than skill 4 will find every tier easier, and the gaps
  between tiers matter more than the exact numbers.
- Think-time limits are wall-clock time, so a busy machine weakens every AI a little. These runs shared the
  machine with other work.
- Within a tier, opponents still vary a lot: Wren is the easiest tier-1 opponent and Pip and Marisol the
  hardest. Grondel is the hardest in tier 2. Unlocks go in `opponents.json` order, so a later change could
  sort each tier from easy to hard.
