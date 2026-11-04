# Card catalog (format 3)

The first calls the platform service answers in Rust (docs/RustMigration.md, phase 2) are card lookups and card
search: `lookupCards`, `searchCards`, `getExpansionSets` and `getBasicLandSets`. Rust doesn't build card views. The Java server writes them
once into a read-only SQLite file, `CatalogExport`, and Rust reads that file the way `CardSearchService` reads the
card repository, step for step.

## Who writes it, and when

- **Image builds.** `Dockerfile.server` starts the server once to build the card database, with
  `-Dxmage.catalog.export=catalog.sqlite`, and ships the file in `cards-prebuilt/` next to `cards.h2.mv.db`. The
  entrypoint installs both into the `cards-db` volume when the image changes.
- **A running server with `XMAGE_GATEWAY=on`.** The entrypoint passes `-Dxmage.catalog.export=cards-db/catalog.sqlite`.
  After startup, a background thread rewrites the file if its stamp doesn't match the card data (about 30 s and
  35 MB for 94,000 printings). It writes `catalog.sqlite.tmp` and renames it, so a reader sees the old file or the
  new one.
- **The gateway** (`XMAGE_PLATFORM_CATALOG`) checks the file every 30 s and opens it again when it changes. Until a
  file is there, it says it doesn't handle these methods, and they go to Java whatever the routes say.

## Tables

| Table | Rows |
|---|---|
| `meta(key, value)` | `stamp` (`<format>:<card content version>:<card code version>:<set content version>`), `exportedAt` (ms), `viewDictionary` (bytes) |
| `printings(id, set_code, card_number, name, view)` | one per printing (set, number and name). `view` is the `CardView` as the bridge sends it (`JsonCodec.GSON`), raw deflate with `viewDictionary` as the preset dictionary |
| `exact(set_code, card_number, printing)` | `CardRepository.findCard(set, number)` for every set and number, main side first |
| `names(query, kind, position, printing)` | kind 0: `findCards(query)` in order; kind 1: printings named exactly `query`; kind 2: `findPreferredCoreExpansionCard(query, "")` |
| `sets(code, standard_legal, release_date)` | every set, for picking a printing when a set is preferred |
| `results(method, json)` | the whole answers of `getExpansionSets` and `getBasicLandSets` |
| `cards(ord, printing, name_rank, …)` | every row a search can return (`splitCardHalf` and `nightCard` false), in the database's row order, with the columns `CardCriteria` filters on, the color and color-identity bits, and the lower-case texts `CardTextPredicate` looks in. `name_rank` is the row's place sorted by name |
| `enums(kind, position, name, text, include_in_search)` | `CardType`, `SuperType`, `SubType` and `Rarity`: the names the JSON uses and the text the database matches (a subtype's description) |
| `formats(name)`, `format_sets(format, position, set_code)` | the deck types, and the sets each constructed one allows |

Keys (`exact`, `names.query`) are lower case: the H2 card database compares text ignoring case (`IGNORECASE=TRUE`),
and Rust lowers what it looks up. The queries cover every name a card has: its name, flip, second-side,
spell-option and back-face names, and each half of an `A // B` name.

## Lookup, as `CardSearchService.findEntry` does it

1. A set and number that are both set: `exact`.
2. No name: null.
3. No set: kind 2 for the name. An `A // B` that isn't a known name falls back to the kind 1 printings of `A`,
   which is the query `findCards` makes next.
4. A set: kind 0 for the name (or kind 1 for `A`, as above). Then `findPreferredOrLatestCard`: the first printing
   from that set, else the newest standard-legal set's, else the newest.

At most 500 entries; anything else in the list counts as no entry. A missing or `null` parameter is error -32602,
as in Java. Every UUID in a view is replaced by a fresh one per answer (the same one wherever the view repeats it),
because Java's views get new ids each time. Shadow comparison treats two UUIDs as equal for the same reason
(`playmat_protocol::compare`).

## Search, as `CardSearchService.search` does it

Rust keeps the `cards` rows in memory (one string for all their text, about 45 MB with the views left on disk) and
filters them as the H2 query `CardCriteria.buildQuery` makes would: LIKE and equality ignoring case, `optimize()`,
the type, supertype, subtype, color and card-number clauses, a format's sets. Then the web refinements (colors,
excluded colors and rarities, mana value ranges, color identity, unique names) and `CardTextPredicate`, with its own
unique-name list, in the same order. Paging is the database's when there are no refinements, else after them.

Without an `ORDER BY` H2 returned rows in whatever order the index it picked gave, so a search could page differently
from one query to the next. `CardSearchService` now asks for `CardCriteria.stableOrder()`: the row order (`_ROWID_`)
after `sortBy`. Rust sorts the same way: row order, or `name_rank` for `sortBy: "name"`.

What Rust doesn't cover it declines (`Outcome::Declined`), and the gateway sends the call to Java as it is, counted in
`playmat_gateway_declined_total`: a sort other than by name, `rules`, `modalDoubleFaced`, `nightCard` other than
false, unknown enum names, fields of the wrong type, a database page of 0, and search tokens with `@`.

## Speed

A 400-card lookup of Career deck names takes 15 ms either way, measured at the client (Rust 15.4 ms p50, Java
15.1 ms). Through the gateway the JVM does no work for it. Most of the Rust time is in sending the 650 kB answer
compressed. Before the preferred printing was exported (format 1), Rust read every printing of each name: basic
lands have hundreds, and the same lookup took 42 ms.

## Checks

- `platform.live.test.ts` also compares 52 searches: what the deck editor, Career, the practice tools and the
  printing picker send, every filter, and the declined cases (2,969 cards; Java 3.5 s, Rust 0.8 s in all).
- `platform.live.test.ts` puts two gateways in front of one Java server, one with these methods on `rust` and one
  on `shadow`. It compares Rust's answers with Java's for every card named in the Career decks and the printings Java
  returned for them (5,046 entries), and for edge cases (case, split, adventure, flip and transform names, unknown sets
  and cards, wrong types, 501 entries). It also checks that `shadow` counted no differences.
- In production, `XMAGE_ROUTE_CATALOG=shadow` runs both and counts by method in
  `playmat_gateway_shadow_method_total`. The group moves to `rust` after a week with no differences.

## Not yet

`deckValidate` (format rules, EDH power level, Commander brackets) stays in Java for now. It is next for this group.
