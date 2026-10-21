# Mage Web Client

The browser client for this XMage fork: sign in, build decks, play the AI or other players, draft and run events, in a Playmat-styled table that aims for MTG Arena-level ease of play. It talks to the server over the JSON-RPC WebSocket bridge in `Mage.Server`.

- What it is for and who it serves: [PRODUCT.md](PRODUCT.md)
- The Playmat design system (tokens, components, rules): [DESIGN.md](DESIGN.md)
- The WebSocket protocol: [../docs/WebSocketAPI.md](../docs/WebSocketAPI.md)
- Deploying the client and server: [../ops/web/README.md](../ops/web/README.md)
- Backlog: the tracker at https://claude.ai/artifact/DAHwcrRiJseP94iUjjhR5A (the old `TODO.md` is archived in [docs/archive](docs/archive/TODO-2026-legacy.md))

## Stack

| Concern | Used |
|---|---|
| UI | React 19, TypeScript, Vite |
| Routing | react-router (data router, lazy routes for the heavier screens) |
| State | Zustand v5 stores; TanStack Query for server lookups (server state, tables, card search) |
| Primitives | Radix (dialog, dropdown menu, tabs, tooltip), lucide icons |
| Styling | CSS Modules on the Playmat tokens in `src/app/styles/tokens.css` |
| Sound | Web Audio (`src/app/match/sound.ts`) |
| Card images | Scryfall, cached in IndexedDB (`src/core/images`) |
| Tests | Vitest (unit in node, components in happy-dom with Testing Library), an accessibility script, Playwright (browser-only e2e and axe in CI; live-server e2e opt-in; visual baselines local) |

## Layout

```text
src/
├── app/            the new app (index.html, served at /)
│   ├── main.tsx    routes, global error hooks
│   ├── screens/    login, home (Play), decks, events, tables, invite (/join), game route
│   ├── match/      the match stage: board, hand, stack, prompts, overlays, card detail, spectator bar
│   ├── decks/      deck builder
│   ├── events/     draft, deck construction, event (tournament) screens
│   ├── social/     chat panel, lobby drawer (chat and players online), profiles, avatars
│   ├── history/    finished matches and saved replays
│   ├── replay/     the replay player
│   ├── stores/     the app's Zustand stores (session, games, decks, play, events, settings, toasts, lobby, social, attention)
│   ├── ui/         shared primitives (Button, Dialog, Field, CardFace, Toaster...)
│   └── styles/     tokens and base CSS
├── core/           framework-free logic, unit tested
│   ├── rpc/        WebSocket JSON-RPC client and event bus
│   ├── game/       game session, interaction model, auto-pass and auto-pay, leaving games
│   ├── decks/      deck types, serializer (.dck/.txt/... import and export), local deck storage
│   ├── social/     chat input commands, chat lines, invite links, sigils and flags
│   ├── replay/     replay timeline (frames, log, turns, playback timing)
│   ├── history/    finished-match summaries
│   ├── images/     card image resolution and cache
│   ├── telemetry/  error reporter (console by default, ring buffer of recent errors)
│   └── headless/   a headless game driver for live tests
└── protocol/generated/   types and API generated from the server's RPC registry (do not edit)
```

The legacy client and its visual harness were deleted in October 2026 (backlog B054); `src/app` is the only UI.

The protocol files are regenerated from the server: `mvn -pl Mage.Server test -Dtest=WebClientApiDocsTest -Dxmage.updateWebApiDocs=true`.

## Bringing a deck

Players bring decks they already have through one import sheet (`src/app/decks/import`), opened from the Decks
shelf, Home, the deck builder's "From a list…" menu, or `/import`:

- **A link** from Archidekt, TCGplayer, MTGTop8, Scryfall or ManaBox: the server reads the deck (`deckImportFromUrl`).
- **A link** from Moxfield, MTGGoldfish, AetherHub, TappedOut or Deckstats: these sites block other apps, so the sheet opens the deck on its site, says where its export is, and finishes when the player pastes the list.
- **A list** in Arena, MTGO, XMage `.dck`, MTGTop8 `.dec` or a site's text export, or a deck file dropped on the shelf.
- **"Send to Playmat"**, a bookmarklet from Settings → Import: on a deck page it reads the deck in the player's own browser and opens `/import#deck=…`. The deck travels in the URL fragment, which no server sees.

⌘V / Ctrl+V anywhere on Decks, Home or the builder (outside a text field) opens the sheet with whatever is on the
clipboard, and dropping a file, link or text on the shelf does the same.

The sheet previews the deck box as it will sit on the shelf, with its curve, card list and a live legality check.
Cards that don't match the card database become amber rows to fix (with a fuzzy printing search) or leave out;
fixes are remembered for the next import. Commanders go into the sideboard, the XMage convention. Imported
decks remember their site, so the Decks screen offers "Update from <site>" with a diff of what changed.

The parsing and matching live in `src/core/deckImport` (pure, unit tested); the site table is
`src/core/deckImport/sites.ts`, and the server's sources are in `Mage.Server/.../websocket/service/deckimport`
(see "Deck import" in `docs/WebSocketAPI.md`).

## Playing together

- **Lobby** (the people button in the rail): the main room's chat and who is online. Whispers (`/w name text`, Tab
  completes the name) reach you anywhere in the app as a toast. `/help` lists the chat commands.
- **Friends and ignores** are kept in this browser, per server: `/friend`, `/ignore` or a player's menu. The server
  learns your ignore list after each login (`chatSetIgnored`) and holds back their chat and whispers; tables you host
  keep ignored players out (`bannedUsers`). Friends coming online show up as a toast.
- **Invites**: "Invite" on your waiting table copies a `/join/<table>` link; "Invite to …" in a player's menu
  whispers it. The link survives signing in first.
- **Table and event chat** sit on your table's row and on the event screen.
- **Profiles**: ratings, match and event record, sigil and flag (yours are sent with your preferences).
- **History**: finished matches on the server, and the replays it keeps (`replayList`). A replay (`/replay/<game>`)
  plays back on the match stage, from your own seat with your hand when you played it: play, pause, step,
  jump by turn, scrub, change speed (Space, ←/→, Shift+←/→).
- **Spectating**: pick whose seat to watch from; players and spectators see how many are watching (`gameWatchers`).
- **Alerts**: while the tab is in the background, its title counts what waits for you (your move, a draft pick, a
  game starting, a whisper); Settings → Alerts adds browser notifications.

## Dev loop

```bash
npm install
npm run dev:all   # builds and starts the Mage server (HTTP 17171, WebSocket 17172) and Vite
```

- New app: http://localhost:5173/
- `Ctrl-C` stops both. If a server survives a failed shutdown, `npm run dev:kill` frees ports 17171 and 17172.
- `npm run dev` starts Vite alone, against a server you run yourself (default `ws://localhost:17172`).

To debug errors in the browser, `window.__mageErrors()` lists the last 50 reported errors.

### Installable app (PWA)

Production builds are installable: `scripts/pwa-plugin.ts` writes `manifest.webmanifest` (named after `VITE_APP_NAME`) and `sw.js`, a hand-written service worker (`scripts/sw.template.js`) that precaches the shell (index.html, every JS and CSS chunk, the Barlow latin fonts, the icons). Navigations go to the network first and fall back to the cached shell, so the app opens offline on the sign-in screen, which then says it is offline. `/ws` and `/img` are never touched. A new release installs in the background; open pages get an "Update available" toast with Reload, otherwise the next load activates it. The dev server registers no service worker. To try it: `npm run build && npx vite preview`. In e2e, service workers are blocked except in `e2e/pwa.spec.ts`.

### Android app

`npm run android` builds the client in the `android` mode (`.env.android`) and packages it with Capacitor into a release
APK (JDK 21 and the Android SDK needed). The app's shell ships inside the APK and runs from `https://localhost`; it plays
on the server named by `PLAYMAT_HOST`, `playmat.deimos.studio` by default, at `wss://<host>/ws` with pictures from
`https://<host>/img`. To build for another server, Multivac for example, set it in the shell:

```bash
PLAYMAT_HOST=playmat.multivac.club npm run android
```

That server must list `https://localhost` in `XMAGE_ALLOWED_ORIGINS`, and if it uses Turnstile, the widget's hostnames
must include `localhost`.

## Scripts and gates

CI (`.github/workflows/web-client.yml`) runs the four gates on every change under `Mage.Web.Client/`:

```bash
npm run typecheck   # tsc -b
npm run lint        # ESLint; zero errors and zero warnings
npm test            # Vitest unit tests, then the accessibility check
npm run build       # tsc -b and the Vite production bundle
```

Lint runs with `--max-warnings 0`: fix a warning rather than raising the cap.

Other scripts:

```bash
npm run test:watch          # Vitest in watch mode
npm run test:e2e            # Playwright specs against the dev server; specs that need a game server skip
npm run test:e2e:ci         # what CI runs: the same specs against `vite preview` (run `npm run build` first)
npm run test:e2e:app        # the new app's live specs (e2e/app-*.spec.ts) against a running local server
npm run test:e2e:import     # deck import against a running local server (MAGE_E2E_NETWORK=1 adds a real Archidekt deck)
npm run size                # bundle-size budget of the last build (also in CI)
npm run preview             # serve the production bundle
```

Playwright's bundled browsers may not match the installed `@playwright/test`; set `PLAYWRIGHT_CHANNEL=chrome` to use the local Chrome.

Unit tests: `*.test.ts` runs in node; `*.test.tsx` runs in happy-dom with `@testing-library/react` (setup in `src/test/setupDom.ts`). A `.ts` test that needs `localStorage` or the DOM starts with `// @vitest-environment happy-dom`. Stores that wire themselves to the connection at import are tested against `src/test/fakeConnection.ts`.

End-to-end: CI runs every spec that needs no game server (sign-in and the axe check of the sign-in screen) against `vite preview`. Specs that play on a real server skip unless `MAGE_E2E_REQUIRE_SERVER=1`; they stay opt-in because CI does not build or start the Java server. To run them, start the server (`npm run dev:server`, or `npm run dev:all` for both) and then:

```bash
PLAYWRIGHT_CHANNEL=chrome npm run test:e2e:app                                    # new app: AI match, decks, tables, full axe pass
PLAYWRIGHT_CHANNEL=chrome MAGE_WEB_E2E_BASE_URL=http://localhost:5173 npm run test:e2e:app  # against a dev server you already run
```

## Languages

The interface speaks English (the source) and Spanish. The code is in `src/app/i18n`, with no library: a small
translator (`translate.ts`) over plain message catalogs, Intl.PluralRules for plurals and Intl.NumberFormat /
DateTimeFormat for numbers and dates. The language is a setting (Settings → Language, saved with the other
settings); "auto" follows the browser. `<html lang>` follows the language. Card names, rules text and anything the
server sends stay in English.

- **English** lives in `i18n/en/*.ts`, one file per area: `core.ts` ships in the entry (shell, sign-in, Play, Decks,
  the `ui` kit); `settings.ts`, `events.ts` and `about.ts` load with their lazy screen, which calls
  `registerMessages(messages)` at module load.
- **Other languages** are one lazy chunk each (`i18n/es.ts`), typed `Catalog`: a missing key is a type error, and
  `catalogs.test.ts` checks every English key and placeholder is there.

Using a string:

```tsx
const t = useT();                                   // in a component: re-renders when the language changes
t('decks.updateFrom', { site: 'Moxfield' });        // "{site}" placeholders; numbers are written the language's way
t('events.minutes', { count: 20 });                 // plural messages: { one: '{count} minute', other: '{count} minutes' }
notify(translate('events.joined'), ...);            // outside React: `t` from '../i18n' (imported as translate in components)
<RichText text={t('decks.empty')} parts={{ keys: <kbd>⌘V</kbd> }} />  // an element inside a sentence
formatNumber(0.6, { style: 'percent' });            // and formatDate(date, options)
```

Adding a string: add the key and English text to the right `en/*.ts` file, then the translation to `es.ts`
(typecheck fails until you do). Keys are `area.thing` (`login.title.signIn`). A key with no message anywhere shows
as the key itself and warns in dev.

Adding a language: add it to `LOCALES` in `i18n/locales.ts` (code and its own name for itself), copy `es.ts` to
`<code>.ts` and translate it, and add its loader to `LOADERS` in `i18n/index.ts`. Plural messages may add the forms
the language needs (`few`, `many`...); a missing form falls back to `other`.

Not translated yet: the match screen, deck builder and importer, draft/build/event screens, admin, social, Tables,
History and replays. They keep English text inline until they move to `t()`.

## Conventions

- Zustand v5: a selector that returns a new array or object on every call loops renders. Select raw slices and derive with `useMemo`.
- Colors, spacing, type and layers come from the tokens (`--z-overlay`, `--z-dialog`, `--z-toast`...), never raw values.
- Pure logic goes in `src/core` with a unit test next to it.

## License

Part of the Mage/XMage project. See the root license files for project terms.
