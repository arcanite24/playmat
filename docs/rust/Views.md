# Views in Rust (step C)

Step C of [GameHost.md](GameHost.md): the Rust engine builds what the web client sees, and it's checked against Java's
until the two agree on real games. A Rust-hosted game needs it (step D), and it moves most of what Java's game thread
does today: building views is 35% of it and sending them 20% (GameHost.md, step A).

## What is compared

The web client gets each `GameView` as `JsonCodec` writes it (`GAME_UPDATE`, and inside `GameClientMessage` with each
question). The comparison takes that JSON, replaces every id by its decision-log reference (`H0.3`, `B12`, `S0`,
`B4a1`…, `DecisionLog.refs`; an id the log can't place becomes `?`), and compares it field by field with the JSON the
Rust engine renders for the same seat at the same point.

- Rust: `playmat_engine::view::game_view(game, seat)` renders the view with references for ids.
- A field's path names it in the report: `players[].battlefield{}.tapped`, `myHand{}.rules`, `stack{}.name`. Lists
  and maps keyed by id are matched by reference, not by position.
- Fields that can't match by design are left out of the comparison and listed in `view::IGNORED` with the reason (the
  priority timer, cache counters).

Building a view must not change the game. `prepareGameView` asks each card whether it's playable, and some of those
checks draw from the game's random numbers, so a client that got views played a different game from one that didn't.
It now draws from a throwaway `Random` while it builds (`GameSessionPlayer.prepareGameView`). Shadow traces record the
random state at each entry (`random`), and a replay that parts from Java's says before which entry the random numbers
parted.

## Steps

1. **Offline, on shadow games.** `ShadowGamesTest` with `-Dxmage.shadowGames.views=<n>` records the asked player's view
   at every n-th answer. `cargo test -p playmat-engine --test views` replays the games and renders its own view at
   each recorded one; with `PLAYMAT_VIEWS=<dir>` it reports the differences per field path, most common first.
   Recorded games with views are kept in `rust/crates/playmat-engine/views/`.
2. **In production, beside the Rust AI's copy.** At each question to the human seat of a game the copy follows, Java
   sends the copy the view it sent the client (already serialized for the socket, ids mapped), the copy renders its own
   at the same point and counts the differing fields. Java logs the counts at the end of the game (`Rust views in …`)
   and the server's metrics add them up.
3. **Done** when the counts stay at zero for a week of real games: step C's gate.

Prompts (the question's message, options, targets, `canPlayObjects`) and the game log's lines are compared the same
way once the views match.
