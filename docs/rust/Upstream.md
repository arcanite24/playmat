# Upstream ledger

Upstream XMage (magefree/mage) keeps changing its cards and its engine, and Playmat merges it in now and then
(`Merge upstream magefree/mage master`, the last on 23 Nov 2026). The Rust cards are translated from the Java ones
(`rust/tools/transpile.py`, docs/rust/Engine.md), so a card upstream changed after its translation can play
differently in Rust than in Java. The ledger records each such change and keeps the cards it touched off the Rust
gate until they are checked again (#191).

## The files

- `rust/upstream/ledger.toml`: the upstream range it covers (`from`, `to`, `synced`), then one `[[change]]` per
  card or engine file the range changed:
  - `kind`: `new card`, `changed card`, `removed card` or `engine`;
  - `names`: the card names the sets print the class under;
  - `scope`: `verified` (on `cards/verified.txt`), `engine` (in the engine, not verified), `pool` (the Pioneer pool
    and the Career decks' cards, `career-pool.txt`; not in the engine) or `outside` (beyond the pool: recorded, not ported);
  - `status`: `pending`, `ported`, `not-needed` or `blocked`, with `why`, the upstream `commit` and its `subject`,
    and the porting `pr` when there is one.
- `rust/crates/playmat-engine/cards/upstream-pending.txt`: the verified cards with a `pending` change. The engine's
  gate (`playmat_engine::verified()`) leaves them out, so a game with one of them plays on Java, and
  `rust/tools/progress.py` doesn't count them.

## After an upstream merge

    python3 rust/tools/upstream_ledger.py scan --from <upstream sha of the last merge> --to <upstream sha now>
    python3 rust/tools/upstream_ledger.py facts-pool /tmp/upstream-pool.txt
    mvn -o -q -pl Mage.Tests test -Dtest=CardFactsTest -Dsurefire.failIfNoSpecifiedTests=false \
      -Dxmage.cardFacts=/tmp/upstream-facts.json -Dxmage.cardFacts.pool=/tmp/upstream-pool.txt
    python3 rust/tools/upstream_ledger.py check --facts /tmp/upstream-facts.json

The upstream sha of a merge is its second parent (`git rev-parse <merge>^2`). `check` translates each changed card
in the pool again with the facts Java writes for it now:

- the same translation as `cards.json`'s: `ported` (nothing for Rust to change);
- a different one, or none any more: `pending`, and a verified card goes on `upstream-pending.txt`;
- not in the engine before or after: `not-needed`.

`check` doesn't change `cards.json`. Engine entries stay `pending` until someone reads them and sets `status` (and
`pr`) by hand.

To port the pending cards, regenerate the cards as docs/rust/Engine.md says (facts, then the transpiler), then run
`upstream_ledger.py requeue`. It takes them off `verified.txt` and `upstream-pending.txt`, so the factory
(`verify_factory.py`) verifies them again with shadow games.

## The weekly report

`.github/workflows/upstream-ledger.yml` runs every Monday (and by hand). It fetches upstream master and lists what
changed since the ledger's `synced` commit in the run's summary, verified cards first. It changes nothing in the
repository.

## 24 Nov 2026: the 23 Nov merge

The cards were last translated on 15 Nov, from upstream as merged on 30 Oct. The 23 Nov merge brought 248 upstream
commits: 402 changed card files, 99 new ones and 131 engine files. `check` on the 200 changed cards in the pool:

- 62 translate the same (`ported`);
- 50 translate differently and 21 no longer translate, so 71 verified cards are `pending` and play on Java until
  they're re-verified;
- 66 weren't in the engine before or after.

A control run over the 13,890 engine cards upstream didn't touch gave the same translation for every one, so all the
differences come from upstream, not from transpiler changes.
