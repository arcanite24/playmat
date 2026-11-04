# Migrating the server to Rust

Investigation of 3 Nov 2026: what it would take to move the Playmat server from Java to Rust one piece at a time, behind toggles, until the Java server can be retired. The goal is server performance: CPU per game, memory per box, and how many games a small VPS can hold.

Short version:

- **The cost is in the rules engine, not the server around it.** After M16–M19, a human game costs about 1.4 CPU-s and an AI game 0.4–0.5 CPU-s with the red mirror, but **8–12 CPU-s with real decks**, and 55% of that is `Game.copy` inside the AI search. Porting the web layer alone saves little CPU. The big win needs a Rust rules engine built for cheap copies.
- **The engine is the hard part.** 282k lines of engine, 32,749 card files (2.05M lines), and upstream adds about 250 cards and changes about 80 engine files every month. About 73% of card files are pure composition of framework classes and could be transpiled; the other 27% (7,600 files, 44% of card code) hold hand-written Java.
- **So the plan is a strangler, in this order:** a Rust gateway in front of the Java server, the non-game services, then a new engine that runs games only when every card in both decks is verified in Rust, with Java as the fallback. AI moves with the engine, because the AI is the engine plus a search loop.
- Every phase ships on its own and has a go/no-go gate. If the engine port stalls, phases 1–3 still pay for themselves and Java keeps running games.

## What we measured

Numbers come from `ops/perf/README.md` (harness: `npm run perf`) and the latest runs in `ops/perf/results/` (3–4 Nov 2026, skill 4, 4 slots).

| Measure | Now |
|---|---|
| Human-paced game vs AI | 1.40–1.44 CPU-s |
| AI vs AI, red mirror | 0.39–0.52 CPU-s |
| AI vs AI, real decks (`MAGE_PERF_DECKS`) | 8.6–12.0 CPU-s, peak RSS 2–3 GB, 3 TB allocated over 84 games |
| Share of real-deck AI CPU in `TwoPlayerDuel.copy` / `GameImpl.<init>` | 54.7% (top leaves: `UUID.equals`, `HashMap.putMapEntries`, `deepCopyObject`, `Effects.<init>`, `Targets.copy`, `ManaCostsImpl.copy`) |
| Largest non-AI cost in a human game | `serializeTree` (views to JSON trees), about 0.37 CPU-s |
| Idle | 41 MB live heap, 345 MB RSS, 30 MB metaspace, ready in 3.5 s |
| 1 vCPU / 1 GB box, 3 games | about 300 MB |
| Held game | 2.8 MB live heap |

What this says:

- On a quiet server the JVM floor (about 200–350 MB RSS) is most of the memory. A Rust gateway plus services would idle at 10–30 MB, but the JVM stays as long as Java runs any games.
- Under load, the AI on real boards dominates everything. One copy of a game deep-copies every player, permanent, effect, target, watcher and cost (3,073 `copy()` methods in the engine, 42,377 in the sets). MAD copies the game for every search node; MCTS copies it for every playout.
- A faster language with the same object model would copy faster, but not by the factor we want. The win is a state model where a copy is cheap (persistent, structurally shared data), which Java could also have but which the existing code can't take without rewriting it, because effects mutate abilities in place (see B225).

## How the server is put together

### Network and sessions

- `Main` builds one `MageServerImpl` (1,282 lines) and one `MainManagerFactory`. JBoss Remoting (17171) starts unless `-Dxmage.remoting=false` (`XMAGE_DESKTOP=false`); the fork's JSON-RPC WebSocket bridge (17172) always starts. Health on 17173, hooks on 17174.
- The bridge (`Mage.Server/.../websocket/`, 63 files, 11k lines) registers **250 methods** in 18 `*Api` classes. Names are flat prefixes (`admin*`, `career*`, `solo*`, `game*`, `room*`, `deckSync*`, `support*`…). `RpcDispatcher` replaces the client's `sessionId` with the connection's own.
- Game updates: engine → `GameController` → `User`/`Session.fireCallback` → `WebSocketCallbackHandler.serializeTree` → `GameStates` (last tree per connection) → `StateDiff` patch → `SerialSender`.
- The browser holds **one multiplexed WebSocket**, so a reverse proxy can't route by method. Caddy routes by path only (`/ws*`, `/hooks/*`, `/public/*`, `/img/*`, the SPA).

### Subsystems and how separable they are

| Area | Size | Bound to the engine? |
|---|---|---|
| admin, decks, moderation, accounts, mail, maintenance, update check | about 5k lines | No. ORMLite/SQLite or H2 CRUD |
| support (Ko-fi, Sponsors, perks) | 2.5k | No, but grants perks into Career |
| rating, social, notify | 650 | No, if match-end events become DTOs |
| record, replay | 1.2k | Consumes match results and views |
| career | 13k | Split: economy, shop, collection and quests are CRUD; match setup and payouts take `mage.game.Game` |
| solo | 3.8k | Yes (saves, forks, what-if, batches) |
| game, draft, tournament, tables, chat, sessions | 13k | Engine core |

### Persistence

H2 1.4.197: `cards.h2` (card DB, read-mostly, shared with the engine), `authorized_user.h2` (accounts). SQLite: `table_record.db`, `user_stats.db`, `web_admin.db`, `web_decks.db`, `web_reports.db` (ORMLite); `career.db`, `solo.db`, `support.db` (raw JDBC). Flat files: replays, saved games, content packs. Every store is a singleton opened by the one process, so moving a store means moving its only writer, not sharing the file.

### Threads

One game thread per running game (`maxGameThreads`, 3–4 on small boxes), blocking on player answers. Separate pools for tournaments, short calls, timeouts, the 32 bridge workers, the senders, and the AI (MAD `xmage.ai.threads`, MCTS one per CPU).

### Engine and cards

- Core: `GameImpl` 4,610 lines, `PlayerImpl` 5,917, `GameState` 1,767, `ContinuousEffects` 1,441, `Combat` 1,960; about 218 event types, 89 watchers, layers 1–7 re-applied by timestamp on every `applyEffects`.
- Framework: 809 effect classes, 294 common and 252 keyword abilities, 221 conditions, 118 dynamic values, 115 costs, 197 filters/predicates, 819 tokens.
- Cards: `LightningBolt` is a constructor that adds `TargetAnyTarget` and `DamageTargetEffect(3)`. 23,959 files (73.2%) look like that. 7,613 files define their own effects or abilities (most often `OneShotEffect`, 5,727 classes), plus 358 watchers and about 1,250 conditions, values and predicates.
- Loading: sets found by class scanning; cards by `Class.forName` from class names (`LazyCardClasses`, M19); game types and players are plugin jars in `PluginClassLoader`.
- Determinism: `GameRandom` is seedable and copies with the game, and `GameOptions.seed` makes the same decks, seed and decisions draw the same cards. There is **no seed + decision log** format yet; replays store views, and saved games are Java serialization.
- Tests: `Mage.Tests` has 7,359 tests, 1,872 classes on `CardTestPlayerBase`. They are already scripts: `addCard`, `castSpell(turn, step, player, "Card$target=…")`, `setChoice`, `assert…`, fed as strings to `TestPlayer`. They can be exported to a neutral format, but assertions read Java objects, so they need a shared state-query layer.

### AI

- MAD (`ComputerPlayer6`, 3.6k lines): depth-limited alpha-beta; every node is `game.createSimulationForAI()` plus a real `activateAbility` and stack resolution; leaves scored by `GameStateEvaluator2`. Budgets: 2,000–5,000 nodes, 3–8 s think time (fd66530381).
- MCTS (1.6k lines): UCT with random playouts on game copies.
- AIs are `Player` subclasses called synchronously with live `Game` objects through about 25 decision methods. An AI outside the JVM can't search without its own engine; only a one-decision-per-prompt policy (LLM, Jev, learned net) can live outside. Hooks for that already exist: `DraftPickSuggester`, `GameAdvisor`, `ComputerPlayerControllableProxy`, the `Archmage` stub.

### Upstream

Monthly merges of `magefree/mage` (6 Oct, 8 Oct, 30 Oct 2026). Since 1 May 2026: 2,329 upstream commits, 2,631 card files changed, 1,505 new cards, 502 engine files changed. A Rust engine either keeps pace with that or ends upstream sync. This plan keeps it, with a translation pipeline and Java as the fallback for cards not yet ported.

## Options considered

| Option | Verdict |
|---|---|
| Big-bang rewrite | No. Years without shipping, and the card tail never ends. |
| JNI/FFM hot paths (evaluator, diffs) in Rust inside the JVM | Little gain: the cost is copying Java object graphs, which native code can't make cheaper from outside. Fine later for pure functions such as state diffs. |
| Run the AI in Rust, rules in Java | Doesn't work: the search needs to copy and step the game thousands of times per decision. |
| Copy-on-write in Java (the deferred B225 follow-up) | Worth doing anyway and much cheaper. It may get 2–3× on AI games. It is the bar the Rust engine has to beat. |
| GraalVM native image | Cuts idle memory and start time, not the copy cost; reflection-heavy card loading makes it a lot of work. Not pursued. |
| **Strangler with a Rust gateway, services, then a new engine gated by card coverage** | **Chosen.** Each step ships, toggles route traffic, Java stays the fallback until it isn't needed. |

## Target architecture

```
browser ── wss /ws ──► Caddy ──► gateway (Rust)  ─── routing table + toggles
                                   │   ├─ sessions, auth, rate limits, origins
                                   │   ├─ per-connection view diff + JSON encode
                                   │   └─ per-method route: java | rust | shadow
                                   │
             ┌─────────────────────┼──────────────────────┐
             ▼                     ▼                      ▼
   platform service (Rust)   Java server (today)     engine host (Rust)
   accounts, admin, decks,   tables, matches,        games whose cards are all
   support, catalog, career  tournaments, Java       verified in Rust; Rust AI
   economy, records          engine and AI           (MAD port, MCTS)
             │                     │                      │
             └──── internal bus (loopback, versioned messages) ────┘
```

- **Gateway**: owns the browser socket. It speaks today's protocol exactly (`Mage.Web.Client/src/protocol/generated/*` is the contract) and forwards each call to a backend by method name. Backends talk to it over an internal channel (first the current JSON-RPC over a loopback WebSocket, later a binary framing such as MessagePack).
- **Routing table** (`routes.toml`, reloadable, overridable by `XMAGE_ROUTE_<GROUP>=java|rust|shadow` and from the admin console): one entry per method group, with optional percentage rollout and staff-only flags. `shadow` sends the call to both, returns Java's answer, and logs differences.
- **Internal bus**: the services need live state the Java server owns today (who is online, sign out on allowlist change, payouts at match end). Java publishes events (`userConnected`, `matchEnded`, `tableCreated`…) and serves a few commands (`disconnectUser`, `grantPerk`); the services subscribe. Every message has a version.
- **Engine per game**: tables get `engine = java | rust | auto`. `auto` picks Rust only when the format, game type and every card in every deck are on the verified list; otherwise Java. Kill switch: `XMAGE_ENGINE=java` sends everything back to Java.

## Phases

Sizes are rough calendar estimates for one developer with agents, measured against the harness at each gate.

### Where it stands (4 Nov 2026)

| Phase | Built | Gate |
|---|---|---|
| 0 | State traces and fingerprints (`mage.game.trace`, a seeded game replays to the same trace), decision logs (`DecisionLog`), the card-test export (`-Dxmage.exportTests`), the wire conformance suite, `playmat-statediff` byte-compatible with Java's patches, the perf harness's `PERF_BACKEND` and Pauper deck suite | Conformance green against Java. A recorded decision log replays to the identical trace on Java twice in a row (`DecisionLogTest`). The Java copy-on-write work isn't started. |
| 1 | JBoss Remoting gone; `playmat-gateway` behind `XMAGE_GATEWAY`, with routes, state patches and Caddy fallback | Conformance green through the gateway; JVM CPU per human game down 12% (1.60 → 1.41 s) for 0.31 s in the gateway; the week on Multivac and Hetzner is still to run. |
| 2 | `playmat-platform`: the card catalog with card search and constructed deck validation ([rust/Catalog.md](rust/Catalog.md)) deck sync ([rust/Decks.md](rust/Decks.md)) and deck import by link ([rust/DeckImport.md](rust/DeckImport.md)) the staff console's stored records ([rust/Admin.md](rust/Admin.md)) and support's records with staff's feedback inbox ([rust/Support.md](rust/Support.md)), each with `shadow`; the accounts moved from H2 to SQLite (`authorized_user.db`, copied once, `XMAGE_ACCOUNTS_DB=h2` copies back); Rust can decline a call it doesn't cover and Java answers it | Both answer as Java does in the live tests, with no shadow differences. Since 4 Nov both groups run in `shadow` on Hetzner for one account (`XMAGE_SHADOW_USERS`). Left with Java: the admin calls that need its memory (live users and tables, the access mode, runtime settings, mail, Discord). Left with Java too: sign-in, registration, mail codes and account deletion (live sessions, in-memory codes and limits), the support status, looks and the Ko-fi and GitHub hooks; the Commander formats' validation stays in Java. |
| 3 | Started: the Career's records that need only `career.db` and the level track (`careerCollection`, `careerDeck`, `careerGameResult`, `careerShowProgress`, `careerLevels`, [rust/Career.md](rust/Career.md)), with `shadow`; Rust reads the Career content from the image's copy of `resources/career`, and serves the starters, cosmetics and opponents and achievements (`careerStarters`, `careerCosmetics`, `careerOpponents`, `careerAchievements`) from the views Java exports | Not met. Next: the rest of the content in Rust (quests), then the deterministic writes, then `matchEnded` from Java for payouts. Packs, capsule pulls, limited drafts and every call that starts a game stay with Java until then. |

### Phase 0: Contracts and measurement (2–3 weeks)

Nothing in Rust yet; this is what makes every later swap safe.

- Wire-protocol conformance suite: replay recorded RPC sessions (requests and events, IDs normalised) against any server; the existing `WebClientApiContractTest`, `WebSocketTransportTest` and the shared `gameStatePatches.json` fixture become cross-language checks.
- **Decision log**: record seed + every player decision + a state fingerprint after each action, for any game (`BatchRunner` and solo seeds are the start). This is the oracle for differential testing of the engine.
- Neutral test export: convert the `CardTestPlayerBase` scripts to JSON/DSL with assertions over a state-query API, so the same tests run against both engines.
- Perf harness gains a `backend` axis and a fixed real-deck suite, so every phase reports CPU-s per game, RSS and p95 prompt latency (the ramp bot) against the same baseline.
- Do the Java copy-on-write work worth doing anyway, so the engine target is measured against the best Java, not today's.

**Gate:** conformance suite green against Java; decision logs replay to identical fingerprints on Java twice in a row.

### Phase 1: Rust gateway (4–6 weeks)

- New Cargo workspace `rust/` (`playmat-gateway`, `playmat-protocol`, later `playmat-platform`, `playmat-engine`), Tokio + axum/tokio-tungstenite, built into its own image, multi-arch like the others.
- Drop the desktop protocol first (decided 3 Nov): JBoss Remoting (17171) and its classes leave the server, so `Session` and `WebSocketCallbackHandler` stop implementing JBoss interfaces.
- Pass-through: the gateway terminates the browser socket, handles origins, per-IP limits, deflate and heartbeats, and forwards to Java's bridge unchanged. Caddy points `/ws*` at it with Java as an instant fallback route.
- Then move the state pipeline: Java sends each view once per game event; the gateway keeps the last tree per connection, diffs, encodes and sends. That takes `serializeTree`, `GameStates` and `StateDiff` off the JVM, and the patches must stay byte-compatible (`gameStatePatches.json`).
- Toggles: `XMAGE_GATEWAY=on|off` in compose, the routing table in place with every group set to `java`.

**Gate:** conformance suite green through the gateway; human-game CPU on the JVM drops by the serialization share; p95 prompt latency no worse; one week on Multivac, then Hetzner.

### Phase 2: Platform service (6–8 weeks)

Move the pure CRUD groups, one routing entry at a time, each through `shadow` before `rust`:

1. `catalog*` and card search: the card DB in Rust (SQLite, exported from `cards.h2` at build time, with an in-memory name index), deck validation by format legality data.
2. `deckSync*`, deck import.
3. `admin*`, staff, invites, audit, settings, moderation reports, client errors.
4. Accounts, sign-in, mail, password reset (H2 `authorized_user.h2` migrated to SQLite once, with a backfill and rollback script).
5. Support hooks (`/hooks/*`, `/public/*` move in Caddy by path).

Each move transfers the single writer of its database file; Java gets a thin client for anything it still reads (staff checks, user lookups).

**Gate:** each group runs a week in `shadow` with zero diffs before `rust`; Java no longer opens those files.

### Phase 3: Career economy, records and ratings (6–8 weeks)

- Career profile, shop, coins, collection, quests, unlocks, cosmetics, gachapon, achievements: `career.db` moves to the platform service.
- Java keeps match setup and publishes `matchEnded` with a DTO (players, decks, result, stats); payouts, XP, ratings (Glicko), table records, user stats and Discord notices run in Rust off that event.
- Solo stays in Java until the engine moves, since it saves and forks live games.

**Gate:** payouts identical in shadow for two weeks of real play; Java's server layer reduced to tables, games, tournaments and solo.

### Phase 4: Rust rules engine (the long one: 6–9 months to a playable core, then a long card tail)

Design for cheap state from day one:

- All game state in persistent, structurally shared collections (`im`/`rpds`-style maps and vectors, or an arena of IDs with copy-on-write chunks). A copy is a pointer bump; an AI node pays only for what its move changes.
- Effects as data plus pure functions over state; layer application as a deterministic pass that can be cached by a change key; events through one queue with replacement, prevention and triggers.
- Cards as data: a card DSL (RON/JSON/Rust macros) that names framework effects, targets, costs, conditions and triggers, mirroring the Java framework classes one to one so cards translate mechanically.
- One seedable RNG in the state (same algorithm as `GameRandom`, so seeds line up where draw order matches).

Card pipeline:

1. **Transpiler** (Java parser over `Mage.Sets`): constructor composition → DSL. Covers the ~24k pure-composition files as framework classes land.
2. **Custom logic** (~7,800 effect classes): LLM-assisted translation into Rust effect functions (no budget cap, decided 3 Nov), each accepted only with its exported tests passing and decision-log games matching.
3. **Upstream intake**: after every monthly merge, the transpiler runs on new and changed cards; anything it can't translate stays Java-only until ported. The coverage list is generated, never hand-edited. Once Java is retired, intake keeps going through the upstream ledger (below).

Running it:

- **Shadow games**: decision logs from real Java games replay in Rust; any fingerprint mismatch is a bug report with the exact action.
- **Coverage-gated routing**: `auto` tables use Rust only when every card is verified. First pool (decided 3 Nov): the **Pauper Career school decks** (23 decks, 218 distinct cards, all commons) in two-player duel, plus the red mirror the harness uses. Pauper first because the cards are simpler, the pool is small and fixed, and most of those games are against the AI, where the CPU goes. Then the Pioneer school decks (about 210 more cards), then Standard, then limited sets.
- Game types: two-player duel first; commander and multiplayer later.

**Gate (first):** all exported tests for the verified card pool pass; 10,000 shadow games of the pool with zero mismatches; CPU-s per AI game on real decks at least 5× better than the Phase 0 Java baseline. Then `auto` for staff, then a share of tables, then default.

### Phase 5: Rust AI (2–3 months, overlaps Phase 4)

- Port MAD's search and `GameStateEvaluator2` onto the Rust engine with node budgets, not wall clock, so results are reproducible; MCTS with real parallel playouts now that copies are cheap.
- Rust AI plays only in Rust-engine games. Archbench (the planned AI benchmark) runs on it first, since AI-vs-AI batches need speed most and need no humans.
- One-decision policies (Jev, LLMs, learned nets via ONNX in `ort`) plug in at the same decision interface.

**Gate:** Rust MAD at least matches Java MAD's results on the bench at equal node budgets, at a fraction of the CPU.

### Phase 6: Retire Java (open-ended)

- Tables, tournaments, drafts and solo move to the Rust engine host once coverage covers the formats people play; Java becomes a fallback pool for unported cards and rare game types.
- Decide then: keep a small Java pool for the long tail for a while, or drop the remaining cards.
- Upstream merges end with Java (decided 3 Nov). Upstream's card and engine changes keep arriving through the ledger instead.
- When nothing routes to Java for a month, remove it from the image.

## Upstream ledger

Decided 3 Nov: the fork may stop merging upstream Java, but it keeps following upstream's card work. Nobody reads upstream by hand; a job does.

- `rust/upstream/ledger.toml` records the last upstream commit synced and one entry per upstream change that touches `Mage.Sets` or `Mage/src`: commit, file, card or engine class, kind (new card, changed card, engine fix), and status (`pending`, `ported`, `not-needed`, `blocked`), with the PR that ported it.
- A scheduled GitHub Actions job (weekly) fetches `magefree/mage`, lists the changes since the last synced commit, and for each card file runs the transpiler. A pure-composition card becomes a ported entry in a PR; anything else becomes a `pending` entry.
- The same job keeps a pinned tracking issue with the checklist of `pending` entries (new cards, changed cards, engine fixes), so porting work is a list to burn down, not a diff to read. Engine fixes link the upstream commit and the Mage.Tests changes that came with it, since those tests become Rust tests.
- Upstream test changes are exported with the Phase 0 converter, so each upstream fix arrives with its check.
- The coverage list the router reads is rebuilt from the ledger: a card upstream changed and we haven't re-ported drops back to Java while Java exists, and is flagged afterwards.

## Toggles at a glance

| Toggle | Where | Values |
|---|---|---|
| `XMAGE_GATEWAY` | compose / Caddy | `on` (gateway in front), `off` (Caddy straight to Java) |
| `XMAGE_ROUTE_<GROUP>` / `routes.toml` / admin console | gateway | `java`, `rust`, `shadow`, plus `%` rollout and `staff` only |
| table `engine` | table options / admin default | `java`, `rust`, `auto` |
| `XMAGE_ENGINE` | server | kill switch: `java` forces every new game onto Java |
| `XMAGE_AI_BACKEND` | engine host | `java`, `rust` (for Rust-engine games and batches) |

## Risks

- **Rules fidelity.** XMage's behaviour is the product of years of bug reports. Mitigation: decision-log differential testing, exported tests, coverage-gated routing, and Java as the fallback per game.
- **Upstream pace.** About 250 cards a month. Mitigation: the transpiler intake and the upstream ledger; unported cards stay on Java, so a monthly merge never blocks.
- **Two systems for a long time.** Every engine fix may need to land twice. Mitigation: keep fork-only engine changes small, and port areas in big batches, not card by card.
- **The engine never reaches enough coverage.** Phases 1–3 still pay off (lighter server layer, faster serialization, cleaner services), and Phase 0's Java copy-on-write work keeps its gains.
- **Data migrations.** Accounts and Career move to new owners. Every migration ships with a backfill, a shadow period and a rollback script, and runs after a backup.

## Decisions (3 Nov 2026)

1. **Desktop client:** not needed. JBoss Remoting is dropped in Phase 1.
2. **First card pool:** the Pauper Career school decks in two-player duel, then Pioneer, then Standard, then limited sets.
3. **Database:** SQLite for every Rust service (`sqlx`, WAL mode, one file per owning service, as today). It needs no extra process on a 1 GB box, backups stay file copies (`ops/web/backup.sh`), and there is one server per deployment. Revisit only if Playmat ever runs more than one server process against the same data.
4. **Upstream:** merging upstream Java may end, but upstream's card and engine changes are tracked automatically in the upstream ledger and ported to Rust.
5. **Translation budget:** no cap on LLM-assisted porting.
