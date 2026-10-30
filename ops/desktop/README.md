# Desktop app

Playmat for one computer, for players who have never heard of Docker, Java or Node: an installer for Windows, macOS
and Linux with everything inside. It starts the XMage server, serves the web client on `http://localhost:17170` and
opens it in the browser. Friends on the same network join at the address the Playmat window shows.

Players download it from the [latest release](https://github.com/arcanite24/playmat/releases/latest):

| | |
|---|---|
| Windows | `Playmat-Setup-Windows-x64.exe`, or in PowerShell `irm https://github.com/arcanite24/playmat/releases/latest/download/install.ps1 \| iex` |
| macOS | `Playmat-macOS-arm64.dmg` / `Playmat-macOS-x64.dmg`, or `curl -fsSL https://github.com/arcanite24/playmat/releases/latest/download/install.sh \| sh` |
| Linux | `curl -fsSL https://github.com/arcanite24/playmat/releases/latest/download/install.sh \| sh`, or `Playmat-Linux-x64.tar.gz` / `-arm64` |

The asset names don't change between versions, so `releases/latest/download/<name>` links always fetch the newest.

## What's inside

```
runtime/                     Eclipse Temurin 17 JRE for the platform (checksum-verified at build time)
app/playmat-launcher.jar     the launcher (launcher/src)
app/server/                  the XMage server release: lib/, plugins/, config/
app/web/                     the built web client (no image proxy: pictures come straight from Scryfall)
app/VERSION
```

On macOS that's `Playmat.app/Contents/Resources/`; on Windows, `%LOCALAPPDATA%\Programs\Playmat` (a per-user
install, so no administrator prompt).

## The launcher

`launcher/src/playmat/launcher`, plain Java 17 with no dependencies:

- **One at a time.** A lock file in the data folder; starting Playmat again just opens the browser.
- **Data folder**, never the install folder (a quarantined macOS app runs from a read-only copy):
  `%LOCALAPPDATA%\Playmat`, `~/Library/Application Support/Playmat` or `~/.local/share/playmat`
  (`PLAYMAT_DATA` overrides it). It holds `server/` (the server's working folder: `db/` with accounts, decks, Career
  and the card database, `saved/` with replays), `logs/` (`launcher.log`, `server.log`) and `settings.properties`.
- **Each start** copies `plugins/` and `config/` from the install into `server/`, renders `config.xml` (accounts on
  or off; browser origins limited to this computer's own addresses on port 17170, plus any in `PLAYMAT_EXTRA_ORIGINS`,
  comma separated, for a tunnel or reverse proxy), and runs `mage.server.Main` as a child process on the bundled Java
  with the installed `lib/` on its class path. A new version (or rebuilt jars) drops the card database, so it's rebuilt
  from the new card pool; everything else in `db/` stays.
- **Ready** when the WebSocket bridge (17172) accepts connections, which the server opens after loading cards. Until
  then, page loads get a "starting" page that reloads itself. The window shows the elapsed time and the server's
  latest log line.
- **Quit** (or closing the window, after a confirmation) stops the server. A server left behind by a launcher that
  was killed is stopped on the next start (`server.pid`, only if it runs from this install's Java).
- **Settings**: `accounts=false` in `settings.properties` (or untick the box in the window) lets players sign in with
  just a name, without Career. `memory=3g` overrides the Java heap, which is otherwise 2 GB on machines with 8 GB of
  RAM or more, less on smaller ones.
- **Updates**: it asks GitHub for the latest release once per start and shows a link when there's a newer one.
- `--headless` runs without a window (status on the console, Ctrl+C stops); `--no-browser` doesn't open the browser.

Ports: 17170 (web client), 17171 (desktop client protocol), 17172 (WebSocket bridge), 17173 (health, loopback only).

## Building

The [Desktop app workflow](../../.github/workflows/desktop.yml) builds every package on each push or PR that touches
this folder, smoke-tests the Linux one (it starts headless and waits for the game server), and publishes a GitHub
release for a `v*` tag:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

or run the workflow by hand with a version. To build locally:

```bash
mvn -B -pl Mage.Server -am install -DskipTests
(cd Mage.Server && mvn -B package assembly:single -DskipTests)
(cd Mage.Web.Client && npm ci && npm run build)
ops/desktop/build-app.sh 0.0.0-dev build
ops/desktop/package.sh linux x64 build dist      # macos needs a Mac, windows needs Inno Setup 6
tar -xzf dist/Playmat-Linux-x64.tar.gz && Playmat/playmat
```

| File | Does |
|---|---|
| `build-app.sh` | server release + web client + launcher jar into `app/` |
| `package.sh` | adds the JRE and wraps `app/` per platform |
| `windows/Playmat.iss` | the Inno Setup installer: per-user, Start menu and desktop shortcuts, starts Playmat when done, offers to delete the data folder on uninstall |
| `macos/` | `Info.plist` and the `Playmat.app` executable; `package.sh` ad-hoc signs the bundle and makes the `.dmg` |
| `linux/playmat` | starts Playmat and adds it to the applications menu on first run |
| `install.sh`, `install.ps1` | the one-line installers, published with each release |
| `tools/Icons.java` | `.ico`, `.iconset` and `.png` icons from the web client's app icon, with only a JDK |
| `RELEASE_NOTES.md` | the release description |

Without an Apple Developer ID or a Windows code-signing certificate, the downloaded installers show a one-time
warning (SmartScreen, Gatekeeper); the release notes walk players through it, and the one-line installers avoid it.
