# Deck sync

The second calls the platform service answers in Rust (docs/RustMigration.md, phase 2): `deckSyncList`,
`deckSyncGet`, `deckSyncPut` and `deckSyncDelete`, the web client's decks kept per account. They are in the `Account`
group (`XMAGE_ROUTE_ACCOUNT`); the group's other calls stay with Java.

## One database, two writers

Rust reads and writes the file Java's `DeckStore` uses, `db/web_decks.db` (`XMAGE_PLATFORM_DECKS`), table `web_deck`
as ORMLite made it. Nothing moves when a route changes, and Java keeps the calls Rust doesn't have: account deletion
and data export (`deleteAll`, `export`). Each Rust call runs in one `BEGIN IMMEDIATE` transaction on one connection, as
Java's store takes one call at a time. The journal mode stays Java's.

## Same rules

- A signed-in user on a server with accounts, else -32001 (the gateway learns the name from `sessionUserName` after a
  login; calls answered in Rust wait for it).
- Deck ids `[A-Za-z0-9_:.-]{1,64}`; names cut to 200 characters; data 1 to 262,144 characters (UTF-16 units, as Java
  counts); 500 live decks per account. Owners are matched in lower case.
- Last write wins by the client's change time: a put or delete older than the stored change is refused with the
  stored `updatedAt`. Deletions stay as tombstones; `deckSyncList` drops those older than 90 days.
- Parameters are read as `RpcCall.string` and Gson's `getAsLong` read them: any JSON primitive is a string; a number
  is cut to a whole one; a string must hold a whole number; an array of one such value counts as that value.
- Gson leaves nulls out: a list entry has no `name` for a deck deleted before the server saw it, and `deckSyncGet` of
  no deck is a reply without `result`.

## Shared decks stay with Java

`deckShareSet`, `deckSharedList` and `deckSharedGet` (#137) are in the same `Account` group but Rust doesn't know
them, so the gateway sends them to Java whatever the route says. Java keeps the shares in its own table,
`web_deck_share`, in the same file, and reads the shared decks from `web_deck` as it is; Rust's deck sync never
touches that table. A deck Rust marks deleted drops out of the shared list the next time Java builds it (and its share
goes), so the two writers can't disagree about what is shared. Porting the three calls would mean reading that table
and the deck JSON summary (`SharedDeck`) the same way.

## Shadow

In `shadow` the gateway asks Rust first, then sends the call to Java. Rust's transaction is rolled back, so it answers
from the state Java is about to change, and Java's change is the one kept.

## Checks

`platform.live.test.ts` (`MAGE_PLATFORM_DECKS=1`, a server with accounts on) runs the same 28 changes and refusals for
one account through Java and another through Rust and compares the answers, then reads the Rust account's decks
through Java. It runs them again through `shadow` and checks that no difference was counted. The crate's unit tests
cover the rules and the rollback.
