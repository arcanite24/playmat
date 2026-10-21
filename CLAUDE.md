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
