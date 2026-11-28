# Deck import

The third group the platform service answers in Rust (docs/RustMigration.md, phase 2): `deckImportFromUrl` and
`deckImportSources`, decks read by link from Archidekt, TCGplayer, MTGTop8, Scryfall and ManaBox. They are the
`DeckImport` group (`XMAGE_ROUTE_DECKIMPORT`). The code is `rust/crates/playmat-platform/src/deckimport/`, a port of
Java's `DeckImportService` and its sources.

## Same rules

- The same sites, links, API addresses and reading of each site's answer. Card names, set codes, sections, cover,
  format and dates come out as Java's `ImportedDeck`, and failures with the same -32005 code, message and
  `{reason}`.
- The same fetcher limits: https on port 443 to the sites' hosts only, redirects followed by hand (three at most,
  allowlisted hosts only), GET with no cookies, an 8 second deadline, a 2 MB cap, the body decoded with the
  response's charset, and the same status and bot-check mapping.
- The same service: a five minute cache of 200 decks, ten imports a minute per connection (cache hits don't count),
  and a site degraded for half an hour after three answers in a row that mean it changed or blocks us.
- `deckImportFromUrl` needs a login. Before one, Rust leaves the call to Java, which refuses it.

## Left to Java

Rust declines the links Java's `java.net.URI` could read differently, so Java answers those: anything outside plain
ASCII, `%` escapes, user info, IPv6 hosts, odd ports, and hosts that aren't a deck site's. Players paste the plain
form.

## The gateway

An import can take seconds, so the gateway answers it on a task of its own. The connection's other calls (pings, game
actions) don't wait for it. In `shadow` both sides fetch the deck, so each import reaches the site twice. The cache,
rate limit and health are kept on each side. With `XMAGE_SHADOW_USERS`, Rust sees only the listed accounts' imports,
so `deckImportSources` can differ if other players' imports degrade a site.

## Checks

- The Rust unit tests read the responses Java's `DeckImportSourcesTest` records (`Mage.Server/src/test/resources/
  deckimport`), with the same assertions. All 21 of those cases, including the failures, gave byte-identical JSON
  from Rust and Java's Gson (4 Nov 2026).
- `platform.live.test.ts` with `MAGE_PLATFORM_DECK_IMPORT=1` reads a deck from each real site, plus links that aren't
  decks, through Java and through Rust, and compares the answers. It also checks that `shadow` counts no differences.
  It is off by default because it reaches the real sites.
