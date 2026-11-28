# Playmat: notes for working on this repository

## Document technical features in the public docs

When a change adds or alters something self-hosters or staff need to know (installing, updating, settings, the admin
console, ops), update the site's Docs section in `site/docs/` in the same PR. `ops/web/README.md` alone isn't enough:
new installs read the site, not the repository.

- The docs are plain HTML, published at https://arcanite24.github.io/playmat/docs/: `index.html` (one card per
  guide), `server.html`, `updates.html` and `admin.html`, styled by `docs.css` on top of `../style.css`.
- To add a guide, copy an existing page, then add it to the `docs-nav` list on every page and to the `.guides` cards
  in `index.html`.
- Keep the docs short and task-first. `ops/web/README.md` stays the full reference, and the docs link to it.
- Write plainly, in the site's voice: a fan project, no marketing.
- `.github/workflows/site.yml` deploys the site on pushes to master that touch `site/**`.
- Say in the PR which docs changed, or why none needed to.

## Game-facing features ship with their Rust side

Java is leaving the game path (docs/rust/Assessment-2026-11-24.md): one-on-one games up to Pioneer, then limited, play
on the Rust host (docs/rust/GameHost.md), and the platform calls answer from Rust (playmat-platform). A change that
players see in a game, or in a call the gateway routes, ships with its Rust side in the same PR:

- a new game option, prompt, assist, table kind or setup is handled by `playmat-host`, or `RustHost` keeps those
  games on Java and logs why ("stays on Java: …"). That reason is a gap to close, not a resting place;
- a new or changed RPC in a routed group gets its playmat-platform handler, or declines on purpose with a comment
  saying why;
- a change to a Java card or engine class is translated again (`rust/tools/transpile.py`), or the card goes on
  `upstream-pending.txt` (docs/rust/Upstream.md).

Say in the PR what its Rust side is, or why it has none.
