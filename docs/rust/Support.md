# Support and feedback

The fifth group the platform service answers in Rust (docs/RustMigration.md, phase 2): support's records in Java's
`db/support.db`, and staff's feedback inbox in `db/web_admin.db`. They are the `Support` and `Accounts` groups
(`XMAGE_ROUTE_SUPPORT`, `XMAGE_ROUTE_ACCOUNTS`). The code is `rust/crates/playmat-platform/src/support.rs`, a port of
the matching parts of Java's `Support` and `SupportStore`, and the feedback calls in `admin.rs`.

## The accounts move to SQLite

Rust can't open H2, and support checks account names. So the server's accounts move from `db/authorized_user.h2` to
`db/authorized_user.db` (SQLite), in `AuthorizedUserRepository.open`:

- The first start without the SQLite file copies every account and remembered device from H2, through a
  `.part` file that is renamed only when the copy is whole and counted. The H2 file isn't touched.
- Names and emails are `COLLATE NOCASE`, as H2's `IGNORECASE=TRUE` matched them. Rust leaves names outside ASCII to
  Java, since NOCASE folds only ASCII letters.
- `XMAGE_ACCOUNTS_DB=h2` is the rollback: the SQLite accounts are copied into H2 and the SQLite file is put aside
  (`authorized_user.db.rolled-back-<time>`), so the next start without the setting copies again.
- The nightly backup copies the SQLite file; a restore of a backup from before the move sets the live file aside, so
  the server copies the restored H2 accounts.

Rust opens the accounts read only, afresh on each call that names an account, and declines the call when the file
is missing (a server on H2, or without accounts).

## What Rust answers

- `supportPerks`, `supportNoticeSeen`, `supportSetWall`: the signed-in account's code (made on the first ask),
  perks, supporter status, thank-you and wall choice.
- `adminSupportLedger`, `adminSupportLink`, `adminSupportRevoke`, `adminSupportGrant`, `adminContributorGrant`:
  staff's (admin role) ledger, with the running GitHub sponsorships' months added as Java adds them, and the links,
  revokes and grants with their audit entries.
- `adminFeedback`, `adminFeedbackMark`: the feedback inbox (moderator role).

Each call runs in one transaction on support.db and web_admin.db; in `shadow` both are rolled back.

## Left to Java

- `supportStatus` and `/public/support.json`: Java keeps the status for 15 seconds. A wall change made in Rust reaches
  it when that runs out.
- `profileLooks`, `profileSetLook`: they need the Career's catalogs of badges and card styles.
- The Ko-fi and GitHub Sponsors hooks: they look up confirmed emails, post to Discord and run the daily job.
- `sendFeedback` and the other accounts calls: sign-in, registration, mail codes and account deletion need live
  sessions, codes kept in memory and Java's rate limits.

## Shadow

A support code made on the first ask is random, and grants made a moment apart differ in their times, so the
comparison allows different `PM-` codes and times up to a minute apart.

## Checks

- `AccountsDbMoveTest` moves accounts to SQLite and back and forward again, with passwords, locks, remembered devices
  and case-insensitive names and emails.
- Unit tests in `support.rs` cover the code, perks, wall and notice, the staff refusals, the ledger with a running
  sponsorship, links and revokes.
- `platform.live.test.ts` with `MAGE_PLATFORM_ADMIN=1` files feedback through Java, marks it in Rust and reads it in
  Java, and compares the ledger and staff's refusals; the shadow test adds four of these calls (4 Nov 2026).
