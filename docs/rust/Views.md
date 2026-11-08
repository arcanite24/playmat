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
2. **In production, beside the Rust AI's copy.** At every 10th question to the human seat of a game the copy follows
   (`XMAGE_RUST_VIEWS_EVERY`), the bridge builds that player's view as the client gets it, ids mapped
   (`mage.server.game.RustViews`, shared with the shadow games), and sends it to the copy (`view`). The copy renders
   its own at the same point and counts the views and fields that differ; a difference never stops it. As it ends it
   reports `views {…}` with the counts, the most common fields and an example of each; Java logs `Rust views in …: n
   compared, m differ` and adds them to the admin console's Overview (`ServerStats.rustAi`).
   `playmat-ai`'s `mirror_views` test plays the recorded games through the copy as Java sends them and checks it
   counts what the replay does. The first live games (6 school-deck games in `RustAiGameTest`) had 40 of 57 views
   matching; these decks' cards showed gaps the meta decks didn't, now fixed offline (below).
3. **Done** when the counts stay at zero for a week of real games: step C's gate.

Prompts (the question's message, options, targets) and the game log's lines are compared the same way once the views
match. `canPlayObjects` is already part of the view comparison.

## Where it stands

On a recording of 40 shadow games of the meta decks (a view every 25 answers): **509 of 539 views match** field for
field. An earlier recording's 552 views went from 334 matching, when the comparison started, to 493. What's left, by
kind:

- Fidelity gaps the views show: a permanent that should have lost its abilities (Tishana's Tidebinder), a state
  trigger Java ran earlier (Mazemind Tome), a delayed trigger's source (Badgermole Cub's earthbend), one life total.
  These are engine fixes, not view fixes.
- Restriction and requirement hints not written yet: "Must block any" (Academic Dispute), "Can't use activated
  abilities" (Clarion Conqueror).
- Playable abilities while a cost is paid: Java leaves out some an X cost could still pay for.
- A few one-offs: an X announced on a permanent's icon, a Room's unlock ability for its other half, Menace's reminder
  text when it's granted (`new MenaceAbility()` has it, `new MenaceAbility(false)` doesn't, and the transpiled card
  doesn't keep which).

On 16 shadow games of the school decks (`ShadowGamesTest`'s default decks, a view every 25 answers): **206 of 220
views match**, from 137. Fixed for them: face-down foretold cards (`Foretell` to the opponent, `Foretell: Name` with
the card as its other face to the owner, each in its own exile zone) and their cast from exile, Crew icons, chosen
names, cards copying others (Mockingbird), Evolved Sleeper's types, regeneration shields, Unlicensed Hearse's count,
"Can't block", Can't Stay Away's exile. The sideboard isn't compared (`view::IGNORED`): the decision log doesn't place
its cards, and play doesn't change it. Left: a Hive of the Eye Tyrant's granted Menace text, a few stack and
playable-ability one-offs.

## How the Rust side gets Java's text

The engine doesn't compute card text. `CardFactsTest` renders, for every card in the pool, Java's own views and
writes them to `cards/views.json`, and `rust/tools/view_templates.py` turns those into templates with relative ids:

- `{name}`: the card's `CardView` in hand; `face:{name}` and `permanent:{name}`: a face's card and its permanent;
  `token:{class}`: a token's; `prepared:{name}`: a prepared permanent's spell copy.
- `spell:{part}`: a part cast on its own (a Room's half, an adventure's spell) as a `Spell`'s `CardView`.
- `abilities:{name}`: each ability's rule, by index; `modes:{name}`: each modal ability's modes' texts, for the
  stack's "Chosen mode" lines.

The view code (`playmat_engine::view`) picks a template and fills in the state: ids, counters, tapped, damage,
attachments, combat, the stack, hints from each ability's `addHint` (transpiled into `hints` and `hint_abilities`),
restriction hints, revealed cards, emblems, playable abilities.

## Ids and short ids

Java's text names objects by their id's first three hex digits (`Llanowar Elves [c3b]`). `ShadowViews` replaces each
by the reference of the object it names, ranked when two share the digits: a card or permanent's own reference (`H`,
`P`, `G`, `B`, `S`, `E`, `C`) first, then a library card's (`L0.3`), then an ability's or face's. Two of the same rank
leave the short id as it is. A spell's card id maps to the spell's stack reference (`S0`).

## Debugging a view

- `PLAYMAT_VIEWS_SHOW=<n>` prints up to n differing values per field path.
- `PLAYMAT_VIEWS_DUMP=game-00012.json:<answer>` prints the Rust view at that answer as `RUST VIEW {json}`, to set
  beside the recorded Java view.
