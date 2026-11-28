# Staff console

The fourth group the platform service answers in Rust (docs/RustMigration.md, phase 2): the staff console's calls
whose records live only in the bridge's `db/web_admin.db` and `db/web_reports.db`. They belong to the `Access`,
`Staff` and `Admin` groups (`XMAGE_ROUTE_ACCESS`, `XMAGE_ROUTE_STAFF`, `XMAGE_ROUTE_ADMIN`). The code is
`rust/crates/playmat-platform/src/admin.rs`, a port of the matching parts of Java's `Staff`, `AdminAudit`,
`AccessPolicy`, `ReportService` and `ClientErrorService`.

## What Rust answers

- `staffMe`, `adminStaff` and `adminAudit`: who is staff, with which role, and the audit log.
- `adminGetReports` and `adminCloseReport`: player reports.
- `adminClientErrors` and `adminClientErrorResolve`: errors the client sent in.
- `adminAllowAdd`, `adminInviteCreate`, `adminInviteRevoke` and `inviteCheck`: the allowlist and invite codes.

## Same rules

- The same roles: the admin console session is the owner, `XMAGE_OWNER` (any case) is the owner, everyone else
  has the role `admin_staff` gives them. Refusals use the same -32001 messages ("Staff only", "This needs the X
  role; yours is Y").
- The same rows and audit entries as Java's ORMLite tables, so either side reads what the other wrote. Rust creates
  the tables if they're missing.
- Each call runs in one transaction on both databases. A database error rolls it back and Rust leaves the call to
  Java.
- Invite codes are 12 random bytes, base64url, as in Java.

## Left to Java

Everything that needs Java's memory: live users and tables, the access mode and notice (`AccessPolicy` caches
them), runtime settings, granting and revoking staff, access requests, mail, Discord, updates and maintenance.
`adminStaff` from the admin console session is declined too, since it lists accounts from Java's H2 database.
Before a login, Rust declines every call but `inviteCheck`, and Java refuses them.

## Shadow

In `shadow`, Rust's change is rolled back and Java makes it. The comparison allows timestamps up to a minute apart
and different invite codes, since both are made fresh on each side.

## Checks

- Unit tests in `admin.rs` cover roles and refusals, invites and the allowlist with their audit entries, reports and
  client errors.
- `platform.live.test.ts` with `MAGE_PLATFORM_ADMIN=1` files a report and a client error through Java, then runs
  the console's calls through Java and through Rust and compares the answers, and checks each side reads the other's
  writes. The shadow test also runs ten admin calls and expects no differences (4 Nov 2026).
