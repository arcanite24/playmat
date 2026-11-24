# Rust migration: where it stands (24 Nov 2026)

Assessment to start a long-running goal for finishing the migration. Sources: production counters on Hetzner
(`saved/gateway-totals.json`, counting since 12 Nov), `rust/tools/progress.py` on master `ff40572`, the server log,
issues #182–#191, and the PRs merged since 12 Nov.

## Headline

The Rust side is built and fast, but **Java still runs about 88% of production games**. Since 12 Nov, Java reported
1,785 finished games (`gameEnded`); the Rust host started 217. Nearly all hosted games are from two accounts
(`neri`, plus one opted-in player). Since 22 Nov the host has started **no games**: the only checks in the log are for
Career tables and saved setups, which stay on Java by design.

The biggest gap isn't the engine anymore. Rollout lags behind, and since 16 Nov new features (Ranked, League,
Ladders, Career modes, events) have gone to the Java side only.

## What's done

| Piece | State |
|---|---|
| Gateway (phase 1) | Carries every connection since 4 Nov. |
| Platform calls (phase 2/3) | 10 groups in `shadow`, **none at `rust`**. 31 methods had no differences in the last 7 days. |
| Engine (phase 4) | 14,024 cards, 14,000 verified (3+ matching shadow games each). 13,913 of 15,259 Pioneer cards. Gate: 9,949/9,955 match. |
| AI (phase 5) | Decision p99 1 ms (Java 1.6 s), 4.3 µs per node (Java 547). ismcts and self-play (#409–#412) merged. |
| Host (phase 6) | AI duels, duels between people, Bo3, timers, spectators, saves and "try again", advice, practice tools. Uses 0.56 CPU-s per game where Java uses 8.6–12. |
| Host reliability | 217 started, 201 over, 14 conceded, **2 without an end**. One (`5b6e1d86`, 16 Nov) stops mid-turn 4, probably cut off by a deploy. No `error` or `panic` ends. |

## What still runs only on Java

**Games the host refuses**
- Career tables (`CareerService.register`): roster duels, puzzles, horde, weekly challenges, gauntlets, Career limited
  runs. Most of the real players' games are here now. Ranked, League and Ladder tables aren't registered as Career
  tables. Whether they qualify for the host hasn't been checked; the log shows reasons only for listed or opted-in
  players.
- Games that start from a setup or a Java save (`GameSnapshot`).
- Anything other than a duel: multiplayer and Commander.
- Limited: drafts, sealed, tournaments, and the League's sealed and draft runs.
- Decks with unverified cards: 1,346 Pioneer cards, and ~18,800 of the 32,849 Java card files outside Pioneer.
- Players who aren't listed or opted in. `XMAGE_RUST_HOST_USERS=neri`, `XMAGE_RUST_OPT_IN=on`, and it isn't the default.

**Calls with no Rust handler** (Java API classes with no Rust counterpart): Session/login, Lobby (tables, matches,
chat), Solo (Java-hosted games), Event, Ladder, League, CareerModes, Home widgets, Settings, MyAccount, WebClient.
Java also keeps the table, the match and the user registry for Rust-hosted games: `RustHost` starts them from
`TableController`.

**Platform groups in shadow that aren't ready**
- Last 7 days with differences: `careerGameResult` 106 of 1,805, `careerQuests` 37/3,797, `searchCards` 4/4,093,
  `getExpansionSets` 1/223, `careerLevels` 1/996.
- Groups that "declined" (left the call to Java): Rust answers only about 12% of `deckValidate` calls (4,722 declined,
  643 compared), so the zero difference count there doesn't yet mean Catalog is ready to go to `rust`.
  `careerQuests` and `staffMe` also decline.
- Career payouts: `CAREERPAYOUTS=shadow`. The two-week gate was never closed out.

**Engine pieces the last Pioneer cards need** (from the tracker): layer 4 type changes and "loses all abilities",
token and counter replacements, a card's own costs, extra steps, and replacements that ask. A few cards can't be
verified because of Java bugs (#289).

## Drift and stale records

- **Upstream ledger (#191) not started**, while upstream keeps landing in Java (#490, 23 Nov, plus the deterministic
  shuffle and discard changes #16810/#16823). Nothing re-verifies Rust cards that upstream changed.
- **No porting since 16 Nov.** The last engine PR was wave 16 (#405), and `cards/` hasn't changed since 17 Nov.
- Tracker (`progress-plan.json`) last updated 15 Nov. #182 still calls phase 3 parked, but the tracker says it was
  unparked on 11 Nov. Its "Where it stands" section is dated 11 Nov.
- Master CI is green, Deploy included.

## Proposed end states for the goal

Chosen 24 Nov 2026: **2, "Java off the game path".**

1. **"Java hosts no 1v1 game up to Pioneer."** Every duel up to Pioneer, Career, Ranked, League and Ladder
   tables included, plays on the Rust host by default, and every platform group is at `rust`. Java stays only for
   out-of-scope formats (multiplayer, Commander, limited, cards outside Pioneer) and for the lobby and session.
   It's countable, it's where the CPU goes, and it fits the 5 Nov target.
2. **Chosen: "Java off the game path."** (1), plus limited (sealed, draft, tournaments: needs the set-by-set card pools)
   and the lobby, session and table in Rust, so a Rust-hosted game never touches Java. Java runs only multiplayer,
   Commander and old cards.
3. **Full retirement (#189).** No Java container. Needs all 32,849 cards, multiplayer and Commander, and the upstream
   ledger working. This takes several quarters, so it works better as a direction than as a goal.

## How progress is counted (all existing counters)

- **Hosted share**: `host_games_started` ÷ `gameEnded` per day in `gateway-totals.json`. Target ≥ 95% of in-scope games.
- **Reliability**: `host_games_ended` with `error` or `panic` at 0, and every started game has an end.
- **Groups at `rust`**: count of `XMAGE_ROUTE_*=rust` in Hetzner `.env`. Each one needs 7 days of `last7.shadow` with
  0 differences and low declines.
- **Cards**: `progress.py` `counted.pioneer.verified`, target 15,259 minus the cards accepted as Java bugs.
- **Why games stay on Java**: the histogram of `Rust host: … stays on Java: <reason>` in the server log, which should
  shrink to out-of-scope reasons only.

- **Limited**: share of sealed, draft and tournament games hosted. Sets with every card verified (a new count in
  `progress.py`).
- **Java on the game path**: calls the gateway sends to Java for a Rust-hosted game, from table creation to the
  record (a new counter). Target 0.

## Steps for the goal (none taken yet)

### Part A: every duel up to Pioneer on Rust

0. **Done 24 Nov.** Hosting works on master `ff40572`: the four Rust-host live suites pass against a local server
   (reconnect, best-of-three, ten soak games, games between people with a spectator, saves and "try again"). Fixed
   along the way:
   - A spectator who started watching before the first board (right as the table started) was refused, so a game
     between people could never be watched from its start (`Hosted::watch`).
   - Deploy drains and restart-when-idle waited only for Java's games, so a restart could cut off a Rust-hosted game
     mid-play. `/healthz` `gamesRunning` now counts both, and `hostedGames` is new. This gives hosted games the same
     drain Java's get; it doesn't end a game nobody is playing, which a long drain still waits out or cuts off.

   The two games with no end are accounted for:
   - `5b6e1d86` (16 Nov) was left for a save on turn 4, carried on as `ca140f0b` and finished. The left game has no
     end record, so it most likely waited until a restart cut it off (inferred: no log line shows the cut-off).
   - The totals count 215 ends where the records have 216: one end was lost from the counters across a restart.
   - Found 24 Nov: a hosted game whose player leaves for good ran until the host's 30-minute limit. By then Java's
     health check had removed its table (about 8 minutes), so the end was lost. #493 ends it as the player's quit
     when Java drops them (3 minutes), as Java does for its own games.

1. Record the state: update #182, the tracker (`progress-plan.json`) and this file's numbers.
2. Decide the rollout default: 217 games without `error` or `panic` meets step E's gate. Deploy the drain fix
   first, then set `XMAGE_RUST_HOST_USERS=*` (owner's OK, production change).
3. **In review: #493** (Career duel, campaign, gauntlet and challenge tables: life, card and permanent edges, the
   opponent's personality, hints counted on Java; Ranked refuses hints and practice tools in #492). A gauntlet
   run's wounds, relics and levels arrive as the setup; the challenge board reads the host's tally. Puzzles and
   trials, horde, pods, limited runs, emblems and timed twists still stay on Java. This week's challenge (W48) has
   19 unverified cards, so it stays on Java until wave 18. Ranked, League and Ladder tables already qualify. Host Career tables: the Career's tally and assists in the host, and setups (twists, nemesis, puzzles). Check
   whether Ranked, League and Ladder tables already qualify, and fix what keeps them on Java.
4. **Decided 24 Nov: no conversion.** "Starts from a setup or a save" is a Java game resumed from a Java save.
   Hosted games already save each turn as a line (`PLAYMAT-HOSTED-LINE`), which resumes on the host
   (`GameOptions.hostedLine`), so once hosting is the default no new Java saves are made. Old ones resume on Java
   until `SoloGames.RETENTION_MS` (30 days) prunes them; step 13 then drops the path. Career setups are step 3's.
5. **In review: #497.** `careerGameResult` answers ranked tables' keys (the likely cause of the 106 differences
   since 21 Nov; inferred, as the logs rotated). `deckValidate` answers Freeform, Freeform Unlimited and Limited
   (most of the 4,802 declines). Each shadow difference is now kept in `differences.log`. `careerQuests`' declines
   are admin accounts, which Rust leaves to Java on purpose. Then: Close the platform diffs (`careerGameResult`, `careerQuests`, `searchCards`), cut `deckValidate` declines, then
   move the groups to `rust` one at a time, a week apart.
6. **Under way: wave 17**, the unverified Pioneer cards the Career, starter and shadow decks use (46). **Wave 18 next:** 121
   cards outside the Pioneer pool that Career decks use. 114 come from the five-paths decks, which every challenge,
   every gauntlet fight and the five-paths campaign play; none of those 30 decks is fully verified now. 15 come from
   three roster decks (Orzhov Oppression, Rakdos Aristocrats, Boros Battalion). Of the roster's 12 decks, 4 are fully
   verified today. The three Commander starters stay on Java as Commander decks. Port the last Pioneer engine pieces, in waves, as before.
7. **In review: #498** (ledger, `check`, a weekly report; 71 verified cards changed upstream play on Java until
   re-verified). Start the upstream ledger (#191) so new upstream changes stop drifting.
8. **In review: #491** (CLAUDE.md and a PR template carry the rule). Add a rule: a new game-facing feature ships with its Rust host side, or with a reason it stays on Java.

### Part B: limited

9. List the sets that sealed, draft, the League and Career limited runs offer, newest first. Port and verify them set
   by set: their commons and uncommons first, then the rest.
10. Host limited games: decks built from a pool, sideboarding between games, best-of-three. A sealed or draft game
    is hosted only when every card of its set is verified.
11. Move the draft and tournament flow to Rust (packs, picks, timers, pairings, rounds, standings), shadowed against
    Java's `DraftController` and `TournamentController` before it takes over.

### Part C: the table, lobby and session in Rust

12. Sessions and login (Session, Access, the user registry, remembered logins), then the lobby: rooms, tables, the
    match, chat, the users list. `RustHost` moves from `TableController` into the gateway, so a Rust game is created
    without Java.
13. The rest of the Java-only API groups, each shadowed for a week first: Solo, Event, Ladder, League, CareerModes,
    Home, Settings, MyAccount, WebClient. Saves of Java games that can't become lines stay playable on Java until
    they expire.
14. Done when Java gets no calls for a game in scope, and runs only multiplayer, Commander and decks with cards Rust
    doesn't have.
