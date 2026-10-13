# XMage WebSocket API

The web client talks to the server over a WebSocket bridge that runs next to the desktop client's JBoss remoting port.
It uses the same server sessions, users and tables as desktop clients, so both kinds of clients can play together.

## Connection

- **URL**: `ws://<server>:17172` (`websocketPort` in `config.xml`). Put the server behind a TLS reverse proxy for `wss://` in production.
- **Protocol**: JSON-RPC 2.0 with positional `params`.
- **Allowed browser origins**: `websocketAllowedOrigins` in `config.xml`, comma separated (for example `https://play.example.com`). Empty or `*` allows any origin. Clients without an `Origin` header (tests, tools) are always allowed.
- **Limits**: 4 MB per message, about 40 requests per second per connection (bursts of 120). Messages are compressed with permessage-deflate when the client supports it.
- Requests of one connection run in order; different connections run in parallel.

### Server settings (Java system properties, `-Dname=value`)

| Property | Default | Meaning |
|----------|---------|---------|
| `xmage.web.trustedProxies` | empty (trust nothing) | Comma separated IPs / CIDR ranges of reverse proxies, e.g. `127.0.0.1,::1,172.16.0.0/12`. Only when the TCP peer is in this list does the server read `X-Forwarded-For` (the client is the right-most address that is not a trusted proxy) or, without it, `X-Real-IP`. The resulting client IP is the session's host (anonymous users are recognized by it on reconnect), the key of the per-IP limits and what the logs show. Behind a proxy this **must** be set, or every client looks like the proxy. |
| `xmage.web.maxConnectionsPerIp` | `16` | Open WebSocket connections per client IP; more are closed with code 1008. `0` = unlimited. Loopback clients are not limited. |
| `xmage.web.maxOutgoingBytes` | `16777216` (16 MB) | Unsent data queued for one connection; a client that stops reading is dropped once it is exceeded. `0` = unlimited. |
| `xmage.adminPassword` (or `-adminPassword=` argument) | empty | Server admin password. `connectAdmin` over WebSocket is refused while it is empty. |

## Sessions

The server issues the session. Logging in (`connectUser`, `connectAdmin`, `authRegister`, `authSendTokenToEmail`, `authResetPassword`) creates a new server session bound to the connection, and every later request on that connection acts as that session.

Methods still take a `sessionId` parameter in the documented position for compatibility, but **the server ignores the value and uses the connection's own session**. A client can never act on another session by sending its id.

Closing the socket counts as a lost connection: the user keeps their tables for a few minutes and can come back from a new connection with `connectUser`, passing the **server-issued restore token** as `restoreSessionId`. Get the token with `sessionGetRestoreToken` right after every successful login (it changes with each login) and keep it client side. A wrong or empty token never takes over another user's seat. Without authentication, a user with the same name from the same client IP is still treated as a reconnect (desktop compatibility).

Access levels in the method table:

- `public`: works before login.
- `login`: creates the connection's session.
- `session`: requires a logged in connection.

## Message format

### Request

```json
{"jsonrpc": "2.0", "id": 1, "method": "roomGetAllTables", "params": ["room-uuid"]}
```

A request without `id` is a notification: it runs, but gets no response.

### Response

```json
{"jsonrpc": "2.0", "id": 1, "result": [ ... ]}
```

### Error

```json
{"jsonrpc": "2.0", "id": 1, "error": {"code": -32602, "message": "Parameter 'roomId' (#0) must be a UUID"}}
```

| Code | Meaning |
|------|---------|
| -32700 | Request is not JSON |
| -32600 | Malformed request (no method, params not an array) |
| -32601 | Unknown method |
| -32602 | Missing or invalid parameter |
| -32603 | Unexpected server error |
| -32000 | The server rejected the request |
| -32001 | Not logged in, or not allowed (wrong admin password, chat not joined) |
| -32003 | Too many requests, or locked out after failed admin logins |
| -32004 | Server busy |
| -32005 | Deck import failed; `data.reason` says why (see Deck import) |

### Server events

The server pushes events as plain objects (no `jsonrpc` field):

```json
{"method": "GAME_UPDATE", "objectId": "game-uuid", "messageId": 12, "data": { ... }}
```

`method` is the `ClientCallbackMethod` enum name. `messageId` increases per session; it restarts at 0 after a new login.

### State patches

Game states are the bulk of the traffic: every `GAME_UPDATE`, question and inform message carries the whole game view
(30 to 100 KB), although little changes between two of them. A client can ask for patches instead:

```json
{"jsonrpc": "2.0", "method": "setCapabilities", "params": [["stateDiffs"]]}
```

Send it right after connecting, before logging in (the setting belongs to the connection; a new connection starts
without it). From then on, every event that holds a game view (`GAME_INIT`, `GAME_UPDATE`, `REPLAY_INIT` and
`REPLAY_UPDATE`, whose `data` is the view, and the messages with `data.gameView`) gets a `state` field:

- `{"seq": 8}`: the view is complete, as before. The first state of each game is always complete.
- `{"seq": 9, "base": 8, "patch": ...}`: the view is left out (`data` is missing, or `data.gameView` is) and is
  `base`'s view with the patch applied. The server only sends a patch when the message gets smaller.

`seq` grows per connection. A patch operation is one of:

| Operation | Meaning |
|-----------|---------|
| `[value]` | replace with `value` (`null` included) |
| `[]` | remove this object key |
| `{"key": operation, ...}` | change an object key by key; other keys keep their value, new keys go after the existing ones |
| `[length, {"index": operation, ...}]` | change an array element by element, then cut or grow it to `length` |
| `["o", {"key": operation or 0, ...}]` | the object gets exactly these keys in this order; `0` keeps the old value |

Applying the patch gives exactly the JSON of the complete view, key order included. When `base` is not the last
state the client has for the game, or the patch does not apply, the client drops the event and calls
`gameStateResync`: the server answers with the last state, complete, as a `GAME_UPDATE` (`REPLAY_UPDATE` for replays)
whose `messageId` is the newest one sent, followed by the open question, if any. `GAME_OVER` and `REPLAY_DONE` are
sent without a state, and after them both sides forget the game; so do `gameWatchStop`, `replayStop`, a new login and
a closed connection. The admin console's statistics (`adminServerStats`, `bridge.state*`) show how much it saves.

### Value conventions

- UUIDs are strings.
- Dates are epoch milliseconds.
- Java views are serialized field by field; `null` fields are omitted.
- `SimpleCardView` objects also carry the card `name`.
- Card views carry `cardId` (the physical card, stable from hand to stack to battlefield), `ownerId` and `controllerId`.

## Methods

Generated from the server's method registry (`mage.server.websocket.api`). Do not edit the table by hand.

<!-- BEGIN GENERATED: methods -->
| Method | Params | Result | Access | Description |
|--------|--------|--------|--------|-------------|
| `ping` | `sessionId?: string`, `pingInfo?: string` | `boolean` | public | Keep-alive. Extends the logged in user's session; always succeeds before login. |
| `setCapabilities` | `capabilities: string[]` | `string[]` | public | Turn on optional protocol features for this connection; returns the ones now on. "stateDiffs": game states are sent as patches against the previous one (see State patches). Call it right after connecting, before logging in; a new connection starts without them. |
| `connectUser` | `userName: string`, `password: string`, `sessionId: string`, `restoreSessionId?: string`, `clientVersion?: string`, `userIdStr?: string`, `rememberToken?: string` | `boolean` | login | Log in. Creates the connection's session; restoreSessionId reattaches a user that lost its connection. rememberToken (from authRememberLogin) stands in for an account's password. |
| `connectAdmin` | `password: string`, `sessionId: string` | `boolean` | login | Log in as server admin. Disabled unless the server was started with a non-empty admin password (-adminPassword=... or -Dxmage.adminPassword=...). After a wrong password the caller's IP is locked out for 1, 2, 4... seconds (at most 5 minutes). |
| `sessionGetRestoreToken` | `sessionId: string` | `string` | session | Token that reattaches this user (and its tables) from a later connection: pass it as connectUser's restoreSessionId. It changes on every login, so fetch it again after each one. |
| `authRememberLogin` | `sessionId: string` | `string` | session | "Keep me signed in": a token that signs this account in from this device without the password (connectUser's rememberToken) until it goes unused for 90 days, the password changes or authForgetLogin. Fails on servers without accounts. |
| `authForgetLogin` | `userName: string`, `rememberToken: string` | `boolean` | public | Signs a remembered device out: its token stops working. Always succeeds. |
| `authRegister` | `sessionId: string`, `userName: string`, `password: string`, `email: string`, `inviteCode?: string` | `boolean` | login | Register a new account (servers with authentication enabled). With a password it becomes the account's password; without one the server emails a generated password. On a contributors-only server the name or email must be on the allowlist, or inviteCode an invite that still works (which puts the account on the list). Fails with the reason. |
| `authSendTokenToEmail` | `sessionId: string`, `email: string` | `boolean` | login | Email a password reset code to an account's address. Succeeds whether or not an account has the address, so addresses can't be probed; fails only when the server can't send mail. |
| `authResetPassword` | `sessionId: string`, `email: string`, `authToken: string`, `password: string` | `boolean` | login | Set a new password with an emailed code (valid 30 minutes, 5 tries). Fails with the reason. |
| `disconnectSession` | `sessionId: string`, `keepGames?: boolean` | `boolean` | session | Log out. With keepGames the user's active tables survive for a later reconnect. |
| `playerLogout` | `sessionId: string`, `keepGames?: boolean` | `boolean` | session | Alias of `disconnectSession`. |
| `connectSetUserData` | `userName?: string`, `sessionId: string`, `userData: UserData`, `clientVersion?: string`, `userIdStr?: string` | `boolean` | session | Store user preferences (avatar, flag, auto-pass settings). userName is ignored; the session identifies the user. |
| `sendFeedback` | `sessionId: string`, `userName: string`, `title: string`, `type: string`, `message: string`, `email?: string` | `boolean` | session | Send feedback to the server's staff (the admin Feedback inbox). type is bug, idea or other; title may be empty; email carries where the app was and its version. 5 an hour per player. |
| `serverGetMainRoomId` | - | `UUID` | public | Id of the main lobby room. |
| `serverGetPromotionMessages` | `sessionId?: string` | `unknown` | public | Server news shown after login. |
| `getServerState` | - | `ServerState` | public | Server version and every game, tournament, player and deck type. Also used as a status probe. |
| `getGameTypes` | - | `GameTypeView[]` | public | Match game types. |
| `getTournamentGameTypes` | - | `GameTypeView[]` | public | Game types usable in tournaments. |
| `getTournamentTypes` | - | `TournamentTypeView[]` | public | Tournament types. |
| `getPlayerTypes` | - | `string[]` | public | Seat types (human and AI players). |
| `getDeckTypes` | - | `string[]` | public | Deck formats. |
| `getDraftCubes` | - | `string[]` | public | Cubes available for cube drafts. |
| `getExpansionSets` | - | `ExpansionSetInfo[]` | public | All card sets. |
| `getBasicLandSets` | - | `BasicLandSetInfo[]` | public | Sets with basic lands, newest first. |
| `searchCards` | `criteria: CardCriteria`, `filters?: WebCardFilters` | `CardView[]` | session | Search the card database (paged with start/count, at most 1000 per page). filters.colorIdentity (letters of WUBRG, "" for colorless) keeps cards a commander of those colors allows. |
| `lookupCards` | `cards: DeckCardInfo[]` | `CardView[]` | public | Card details for deck entries (by set and number, falling back to name), in the same order; null for unknown cards. At most 500 per call. |
| `deckValidate` | `deckType: string`, `deck: DeckCardLists` | `DeckValidationResult` | session | Check a deck against a format, with EDH power level and Commander brackets. |
| `deckImportFromUrl` | `url: string` | `ImportedDeck` | session | Read the deck behind a deck website link. Ten imports a minute per session; results are cached for five minutes. Fails with code -32005 and data {reason}: private, not_found, blocked, rate_limited, site_changed, unsupported, too_large, timeout or unavailable. |
| `deckImportSources` | - | `DeckSourceStatus[]` | public | Deck websites the server imports from by link, and whether each works right now (ok or degraded). |
| `roomGetUsers` | `roomId: UUID` | `RoomUsersView[]` | session | Users and server statistics of a room. |
| `roomGetAllTables` | `roomId: UUID` | `TableView[]` | session | All open and running tables of a room. |
| `roomGetTableById` | `roomId: UUID`, `tableId: UUID` | `TableView \| null` | session | One table of a room. |
| `roomGetFinishedMatches` | `roomId: UUID` | `MatchView[]` | session | Recently finished matches of a room. |
| `roomCreateTable` | `sessionId: string`, `roomId: UUID`, `options: WebMatchOptions` | `TableView` | session | Create a match table. The creator still has to join a seat. |
| `roomJoinTable` | `sessionId: string`, `roomId: UUID`, `tableId: UUID`, `name: string`, `playerType: string`, `skill: number`, `deck: DeckCardLists`, `password?: string` | `boolean` | session | Take a seat at a table (yourself or an AI player). |
| `roomLeaveTableOrTournament` | `sessionId: string`, `roomId: UUID`, `tableId: UUID` | `boolean` | session | Leave a table or tournament before it starts. |
| `roomWatchTable` | `sessionId: string`, `roomId: UUID`, `tableId: UUID` | `boolean` | session | Spectate a running match. |
| `matchStart` | `sessionId: string`, `roomId: UUID`, `tableId: UUID` | `boolean` | session | Start a match once all seats are filled (table owner only). |
| `tableSwapSeats` | `sessionId: string`, `roomId: UUID`, `tableId: UUID`, `seatNum1: number`, `seatNum2: number` | `boolean` | session | Reorder seats (table owner only). |
| `tableRemove` | `sessionId: string`, `roomId: UUID`, `tableId: UUID` | `boolean` | session | Remove a table (table owner only). |
| `tableIsOwner` | `sessionId: string`, `roomId: UUID`, `tableId: UUID` | `boolean` | session | Whether the caller owns the table. |
| `deckSubmit` | `sessionId: string`, `tableId: UUID`, `deck: DeckCardLists` | `boolean` | session | Submit a deck for a table in construction or sideboarding. |
| `submitDeck` | `sessionId: string`, `tableId: UUID`, `deck: DeckCardLists` | `boolean` | session | Alias of `deckSubmit`. |
| `deckSave` | `sessionId: string`, `tableId: UUID`, `deck: DeckCardLists` | `boolean` | session | Save the in-progress limited deck without submitting it. |
| `chatJoin` | `chatId: UUID`, `sessionId: string`, `userName?: string` | `boolean` | session | Subscribe to a chat. |
| `chatLeave` | `chatId: UUID`, `sessionId: string` | `boolean` | session | Unsubscribe from a chat. |
| `chatSendMessage` | `chatId: UUID`, `userName?: string`, `message: string` | `boolean` | session | Post to a chat as the logged in user, who must have joined it (chatJoin; the lobby chat is joined on login). userName is ignored; the session decides the author. |
| `chatSetIgnored` | `names: string[]` | `boolean` | session | Players this user ignores, by name, for as long as the session lasts (the client keeps the list and sends it after each login): their table chat doesn't reach the user and their whispers are refused. At most 500 names; an empty list clears it. |
| `chatFindByGame` | `gameId: UUID` | `UUID \| null` | session | Chat of a game. |
| `chatFindByTable` | `tableId: UUID` | `UUID \| null` | session | Chat of a table. |
| `chatFindByTournament` | `tournamentId: UUID` | `UUID \| null` | session | Chat of a tournament. |
| `chatFindByRoom` | `roomId: UUID` | `UUID \| null` | session | Chat of a room. |
| `gameJoin` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Join a started game as a player (after START_GAME). Also used to resync after a reconnect. |
| `gameResync` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Send this connection the game's open question again, when a reply or the question seems lost. Returns false when nothing is waiting for the player's answer. |
| `gameStateResync` | `gameId: UUID`, `sessionId: string` | `boolean` | session | With state patches on (setCapabilities): the client could not apply a patch, so the server sends the game's last state complete (as GAME_UPDATE or REPLAY_UPDATE), then its open question. Returns false when it has none; the next state is sent complete anyway. |
| `matchQuit` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Concede the whole match. |
| `gameWatchStart` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Start spectating a game. |
| `replayList` | - | `ReplayInfo[]` | session | Recorded games available for replay, newest first. Games are recorded as a spectator sees them (xmage.replays, on by default) and kept for xmage.replays.retentionDays (30). |
| `replayPage` | `gameId: UUID`, `start: number`, `count: number` | `ReplayPage \| null` | session | Events of a recorded game, in order: board snapshots ({t, view, myHand?}) and game log lines ({t, log}). At most 200 per page; myHand is the caller's own hand in games they played. Null when there is no such replay. |
| `gameWatchers` | `gameId: UUID` | `string[]` | session | Names of the users spectating a running game (empty when the game is over or unknown). |
| `gameWatchStop` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Stop spectating a game. |
| `sendPlayerUUID` | `gameId: UUID`, `sessionId: string`, `data?: UUID` | `boolean` | session | Answer a prompt with an object or player id (targets, choices, attackers, blockers). |
| `sendPlayerString` | `gameId: UUID`, `sessionId: string`, `data: string` | `boolean` | session | Answer a prompt with text (choices, "special" mana payment). |
| `sendPlayerBoolean` | `gameId: UUID`, `sessionId: string`, `data: boolean` | `boolean` | session | Answer a yes/no prompt, pass priority (false) or cancel mana payment (any value). |
| `sendPlayerInteger` | `gameId: UUID`, `sessionId: string`, `data: number` | `boolean` | session | Answer an amount prompt. |
| `sendPlayerManaType` | `gameId: UUID`, `playerId: UUID`, `sessionId: string`, `manaType: ManaType` | `boolean` | session | Pay from the mana pool with one mana type. |
| `sendPlayerAction` | `action: PlayerAction`, `gameId: UUID`, `sessionId: string`, `data?: unknown` | `boolean` | session | Send a player action: pass modes (F-keys), concede, undo, rollback, auto-answer and trigger-order settings. |
| `cheatShow` | `gameId: UUID`, `sessionId: string`, `playerId: UUID` | `boolean` | session | Test mode only: reveal a player's library. |
| `gamePractice` | `gameId: UUID`, `sessionId: string`, `tool: string`, `cardName?: string`, `amount?: number` | `boolean` | session | Practice tools in a game against the AI only: tool HAND or BATTLEFIELD (cardName, amount cards), DRAW (amount), UNTAP_ALL, LIFE (amount). Runs the next time you are asked something; announced in the game log. |
| `replayInit` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Open a saved game for replay. |
| `replayStart` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Start the replay. |
| `replayStop` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Stop the replay. |
| `replayNext` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Step forward. |
| `replayPrevious` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Step back. |
| `replaySkipForward` | `gameId: UUID`, `sessionId: string`, `moves: number` | `boolean` | session | Skip several steps forward. |
| `roomCreateTournament` | `sessionId: string`, `roomId: UUID`, `options: WebTournamentOptions` | `TableView` | session | Create a tournament table. |
| `roomJoinTournament` | `sessionId: string`, `roomId: UUID`, `tableId: UUID`, `name: string`, `playerType: string`, `skill: number`, `deck: DeckCardLists`, `password?: string` | `boolean` | session | Join a tournament (yourself or an AI player). |
| `roomWatchTournament` | `sessionId: string`, `tableId: UUID` | `boolean` | session | Spectate a tournament. |
| `tournamentStart` | `sessionId: string`, `roomId: UUID`, `tableId: UUID` | `boolean` | session | Start a tournament (owner only). |
| `tournamentJoin` | `tournamentId: UUID`, `sessionId: string` | `boolean` | session | Open a started tournament (after START_TOURNAMENT). |
| `tournamentQuit` | `tournamentId: UUID`, `sessionId: string` | `boolean` | session | Leave a running tournament. |
| `tournamentFindById` | `tournamentId: UUID` | `TournamentView \| null` | session | Tournament standings and rounds. |
| `draftJoin` | `draftId: UUID`, `sessionId: string` | `boolean` | session | Open a started draft (after START_DRAFT). |
| `draftQuit` | `draftId: UUID`, `sessionId: string` | `boolean` | session | Leave a running draft. |
| `sendDraftCardPick` | `draftId: UUID`, `sessionId: string`, `cardId: UUID`, `hiddenCards?: UUID[]` | `DraftPickView \| null` | session | Pick a card from the current booster. |
| `sendDraftCardMark` | `draftId: UUID`, `sessionId: string`, `cardId?: UUID` | `boolean` | session | Mark (or unmark with null) the card to auto-pick when the timer runs out. |
| `draftSetBoosterLoaded` | `draftId: UUID`, `sessionId: string` | `boolean` | session | Tell the server the booster is on screen, so the pick timer may start. |
| `adminGetUsers` | `sessionId: string` | `UserView[]` | session | Staff (moderator): all connected users. |
| `adminDisconnectUser` | `sessionId: string`, `userSessionId: string`, `reason?: string` | `boolean` | session | Staff (moderator): sign a player's session out. |
| `adminEndUserSession` | `sessionId: string`, `userSessionId: string`, `reason?: string` | `boolean` | session | Staff (moderator): end a player's session; they leave their tables. |
| `adminMuteUser` | `sessionId: string`, `userName: string`, `durationMinutes: number`, `reason?: string` | `boolean` | session | Staff (moderator): mute a player in chats, online or not; 0 minutes lifts the mute. |
| `adminLockUser` | `sessionId: string`, `userName: string`, `durationMinutes: number`, `reason?: string` | `boolean` | session | Staff (admin): lock an account out, online or not; 0 minutes unlocks it. |
| `adminActivateUser` | `sessionId: string`, `userName: string`, `active: boolean`, `reason?: string` | `boolean` | session | Staff (admin): activate or deactivate an account. |
| `adminToggleActivateUser` | `sessionId: string`, `userName: string` | `boolean` | session | Staff (admin): toggle an online player's active state. |
| `adminTableRemove` | `sessionId: string`, `tableId: UUID`, `reason?: string` | `boolean` | session | Staff (admin): remove any table. False when it was gone already. |
| `adminTableWatch` | `sessionId: string`, `tableId: UUID` | `string` | session | Staff (moderator): watch any running game from your own player session, whatever its spectator settings; the game opens through the usual watch callback. Returns the game id, or null when there is no running game or you play in it. |
| `adminSendBroadcastMessage` | `sessionId: string`, `message: string` | `boolean` | session | Staff (admin): message every connected player; a message starting with "warn" shows in red. |
| `adminServerStats` | - | `ServerStats` | session | Staff (moderator): load and traffic: users, tables, games, memory, CPU, and the size of messages sent to web clients (with the heaviest callbacks and every one over 256 KB). |
| `adminGetReports` | `openOnly?: boolean` | `PlayerReport[]` | session | Staff (moderator): player reports, newest first; only the open ones unless openOnly is false. |
| `adminCloseReport` | `reportId: number`, `resolution: string` | `boolean` | session | Staff (moderator): close a report with a note about what was done. False when it was closed already. |
| `adminClientErrors` | `includeResolved?: boolean` | `ClientErrorGroup[]` | session | Staff (moderator): crash reports from web clients, one row per kind of error with how often it happened, seen most recently first; resolved ones only when asked for. |
| `adminClientErrorResolve` | `id: number`, `resolved?: boolean` | `ClientErrorGroup` | session | Staff (moderator): mark a kind of client error fixed (or, with resolved false, open again). It opens again by itself when it happens again. |
| `testEndGame` | `sessionId: string`, `tableId: UUID` | `boolean` | session | Test mode only: end the running game of a table you own. |
| `testConcedeMatch` | `sessionId: string`, `tableId: UUID`, `losingPlayerIndex?: number` | `boolean` | session | Test mode only: make one player of a table you own concede the match. |
| `adminAudit` | `actor?: string`, `action?: string`, `target?: string`, `before?: number` | `AuditEntry[]` | session | Staff: the audit log, newest first, 100 at a time. action matches a family ("access" covers "access.mode"); before is the id to page back from. |
| `adminAccess` | - | `AccessOverview` | session | Staff: the access mode, notice, allowlist, invites and access requests. |
| `adminAccessPreview` | `mode: string` | `string[]` | session | Staff: who is online now and would be signed out by switching to this mode (open or allowlist). |
| `adminSetAccessMode` | `mode: string`, `reason: string` | `string[]` | session | Staff: switch between open and allowlist. Switching to allowlist signs out whoever isn't on it; returns their names. Needs accounts on the server. |
| `adminAllowAdd` | `nameOrEmail: string`, `note?: string` | `AllowEntry` | session | Staff: allow an account name, or an email address (any account registered with it). |
| `adminAllowRemovePreview` | `entryId: number` | `string[]` | session | Staff: who is online now and would be signed out by taking this entry off the allowlist. |
| `adminAllowRemove` | `entryId: number`, `reason: string` | `string[]` | session | Staff: take an entry off the allowlist; whoever it let in is signed out. Returns their names. |
| `adminInviteCreate` | `note?: string`, `maxUses?: number`, `days?: number` | `Invite` | session | Staff: a new invite link code (/join?invite=code). It registers an account and allowlists it; 1 use and 7 days unless asked otherwise (at most 100 uses and 90 days). |
| `adminInviteRevoke` | `code: string` | `boolean` | session | Staff: stop an invite link from working. False when it was revoked already or doesn't exist. |
| `adminAccessDecide` | `requestId: number`, `approve: boolean` | `AccessRequest \| null` | session | Staff: approve an access request (its email address goes on the allowlist, and the person is emailed when the server has mail) or decline it. Null when it was decided already. |
| `adminSetNotice` | `notice: string` | `boolean` | session | Staff: the notice shown on sign-in and Home, e.g. "builds from master, data may be reset". Empty clears it. At most 500 characters. |
| `inviteCheck` | `code: string` | `InviteCheck` | public | Whether an invite link still works. Callable before login. |
| `githubStart` | - | `string` | public | Sign in with GitHub: the GitHub page to send the player to. GitHub sends them back to /auth/github?code=...&state=..., for githubFinish. Fails when it isn't set up. |
| `githubFinish` | `state: string`, `code: string`, `userName?: string` | `GithubSignInResult` | public | Sign in with GitHub, back from GitHub: the account to sign in to with its rememberToken (connectUser's), or a suggestedName when a new account needs a name (call again with userName). On a contributors server, the repository's people and allowlisted emails get in. |
| `accessRequest` | `userName: string`, `email: string`, `note?: string` | `boolean` | public | Ask to be let in to a contributors-only server, or for an account while registration is invite only: staff see it in /admin. 3 an hour from one address. Callable before login. |
| `staffMe` | - | `StaffMe` | session | The calling session's staff role, or a null role for players. |
| `adminStaff` | - | `StaffList` | session | Staff (owner): who has a staff role. |
| `adminStaffGrant` | `userName: string`, `role: string`, `reason?: string` | `StaffMember` | session | Staff (owner): give an account a role (moderator, admin or owner), or change it. |
| `adminStaffRevoke` | `userName: string`, `reason?: string` | `boolean` | session | Staff (owner): take an account's role away. False when it had none. |
| `adminAccounts` | `search?: string`, `offset?: number` | `AccountsPage` | session | Staff (moderator): registered accounts whose name or email contains search, by name, 50 at a time from offset. |
| `adminAccount` | `userName: string` | `AccountDetail` | session | Staff (moderator): one account with its sessions, stats, reports by and about it, its feedback and its audit history. |
| `adminResetCode` | `userName: string`, `reason?: string` | `StaffResetCode` | session | Staff (admin): a one-time password reset code for the account, shown once; it is emailed too when the server can send mail. The player enters it under Forgot password with the account's email. |
| `adminAccountExport` | `userName: string` | `string` | session | Staff (admin): everything the server keeps about the account as JSON, for the player or before a deletion. Passwords and sign-in tokens are left out. |
| `adminDeleteAccount` | `userName: string`, `reason: string` | `boolean` | session | Staff (admin): delete the account with its decks, Career, reports, stats, feedback, allowlist entry and staff role, signing its player out. Can't be undone: export first. The audit log keeps the record of the deletion. |
| `adminFeedback` | `status?: string` | `FeedbackEntry[]` | session | Staff (moderator): feedback players sent, newest first, 200 at most; status new, seen or done narrows it. |
| `adminFeedbackMark` | `id: number`, `status: string`, `note?: string` | `FeedbackEntry` | session | Staff (moderator): mark feedback new, seen or done, with an optional note (such as the issue it became). |
| `adminSettings` | - | `ServerSettings` | session | Staff (admin): settings that change without a restart (registration, Career, AI opponents, batch games, time to answer, Discord posts), each with where its value comes from; the Discord webhooks; planned maintenance. |
| `adminSetSetting` | `name: string`, `value: string`, `reason?: string` | `ServerSettings` | session | Staff (admin): change a setting; an empty value goes back to the server's config. |
| `adminMaintenance` | `minutes: number`, `message?: string`, `restartWhenIdle?: boolean` | `ServerSettings` | session | Staff (owner): announce maintenance in so many minutes (0: now). New tables stop at once and players are warned; with restartWhenIdle the server exits once the time has come and no game is running, and its restart policy brings it back. A new announcement replaces the old one. |
| `adminMaintenanceCancel` | - | `ServerSettings` | session | Staff (owner): call planned maintenance off; tables can start again. |
| `adminDiscordTest` | `channel: string` | `string` | session | Staff (admin): post a test message to the lobby or staff channel now, and say what Discord answered. |
| `serverInfo` | - | `ServerInfo` | public | Whether the server has accounts and mail, and its name and password rules. Callable before login. |
| `deckSyncList` | - | `DeckSyncEntry[]` | session | Every deck kept on the server for the signed-in account, without the deck data, deleted ones included (as tombstones, for 90 days). Only on servers with accounts. |
| `deckSyncGet` | `deckId: string` | `DeckSyncEntry \| null` | session | One deck of the signed-in account with its data, or null when there is none. |
| `deckSyncPut` | `deckId: string`, `name: string`, `updatedAt: number`, `data: string` | `DeckPutResult` | session | Store a deck for the signed-in account. The newer change wins: when the server's copy is newer, nothing is stored and its updatedAt comes back. Up to 500 decks of 256 KB each. |
| `deckSyncDelete` | `deckId: string`, `deletedAt: number` | `DeckPutResult` | session | Delete a deck of the signed-in account, unless the server has a change newer than the deletion. |
| `reportPlayer` | `userName: string`, `reason: string`, `details: string`, `gameId?: string` | `number` | session | Report a player to the server's moderators. Reasons: abuse, cheating, spam, stalling, other. A player can have 5 open reports at a time. Returns the report id. |
| `clientReportError` | `report: ClientError` | `boolean` | public | A crash report from the web client: kept in the server log and the admin console, never sent elsewhere. Limited per connection; extra reports are dropped. |
| `careerState` | - | `CareerState` | session | Whether this server offers Career, and the signed-in account's Career (profile null before it opens one). |
| `careerStarters` | - | `CareerStarter[]` | session | The starter decks a Career can open with. |
| `careerStart` | `starterId: string` | `CareerProfile` | session | Open a Career with a starter deck: its cards become the collection, plus coins for a first pack. |
| `careerCollection` | - | `CareerCard[]` | session | Every card the account owns in Career, by printing. Basic lands are free and not listed. |
| `careerOpponents` | - | `CareerOpponent[]` | session | The AI opponents in tiers, with the account's wins against each and which are open. A tier opens after enough wins in the one before. |
| `careerPlay` | `opponentId: string`, `deck: DeckCardLists`, `adaptive?: boolean` | `CareerMatch` | session | Start a Career match against an open opponent with a deck of owned cards (basic lands are free). The server sets up the table and starts the match; the game arrives like any other. Adaptive: the AI's skill follows your hidden Career rating instead of the tier's, to keep your win rate near 55%. |
| `careerShop` | - | `CareerShopSet[]` | session | The sets the Career shop sells packs of, newest first, with the price in coins: the newest sets, and the ones campaign chapters opened. Sets a chapter still has to open come last, locked, saying which. |
| `careerBuyPack` | `setCode: string` | `CareerPackResult` | session | Buy and open a pack. Copies past four of a card become coins (commons, uncommons) or a wildcard (rares, mythics); those cards say so in convertedTo. |
| `careerCraft` | `setCode: string`, `cardNumber: string` | `CareerProfile` | session | Craft one copy of a card: commons and uncommons cost coins, rares and mythics a wildcard of their rarity. |
| `careerExport` | - | `string` | session | The account's whole Career as a signed text file, to keep a copy. |
| `careerImport` | `data: string` | `CareerProfile` | session | Bring back an exported Career, for an account that has none. Only files this server made for this account are taken. |
| `careerQuests` | - | `CareerQuests \| null` | session | The quests waiting (up to three; a new one arrives each day) and whether one can be swapped today. Null before the account opens a Career. |
| `careerRerollQuest` | `slot: number` | `CareerQuests` | session | Swap the quest in a slot for a different one, once a day. The new quest starts from zero. |
| `careerAchievements` | - | `CareerAchievement[]` | session | Every achievement, earned or not, with progress on the Career-long ones. A secret one has no name or text until it's earned. |
| `careerLevels` | - | `CareerLevelTrack \| null` | session | The level track: the account's level and XP, what every level gives, and the cosmetics unlocked so far. |
| `careerWeekly` | - | `CareerWeekly` | session | Career wins this week (ISO weeks, UTC) and the goals that each give a free pack. |
| `careerGameResult` | `gameKey?: string` | `CareerGameResult \| null` | session | What a finished game paid: the match's coins and XP, quest progress, achievements, level rewards and the weekly goal. The key is the Career table id (or the game id of a regular AI game); without one, the latest game. Null when that game paid nothing. |
| `careerSetProgress` | - | `CareerSetProgress[]` | session | How much of each set the collection has: different cards owned (any printing) out of the cards its packs hold, basic lands left out. Most complete first. |
| `careerCountAiGames` | `count: boolean` | `CareerProfile` | session | Whether regular games against the AI also count for quests and achievements (off by default). They never pay coins for the match itself, and practice, goldfish and cheat games never count. |
| `careerCampaigns` | - | `CareerCampaign[]` | session | Every campaign with its chapters and nodes, each locked, open or done for this account. |
| `careerCampaignPlay` | `campaignId: string`, `nodeId: string` | `CareerMatch` | session | Play a campaign duel or trial that's open (or done, to play it again), with the node's deck or the chapter's, under the school's format: a pod seats every AI, a final is a best of three. A first win pays the node's reward. |
| `careerCampaignDeck` | `campaignId: string`, `nodeId: string` | `DeckCardLists` | session | The list of the deck you play at a campaign duel, to read before it. |
| `careerCampaignChoose` | `campaignId: string`, `nodeId: string`, `optionId: string` | `CareerGameResult` | session | Take one option at an open choice node; it's paid at once and the path moves on. |
| `careerPuzzles` | - | `CareerPuzzle[]` | session | The puzzle book in order: each puzzle's position, stars and attempts. A puzzle opens when the one before it is solved; the first three are always open. |
| `careerPuzzlePlay` | `puzzleId: string` | `CareerMatch` | session | Try a puzzle: the game starts from its position on your turn, and you lose at the end of the turn. Stars go by attempts: three on the first try, two within three. |
| `careerDailyPuzzle` | - | `CareerDailyPuzzle` | session | Today's puzzle, the same for everyone on the server (UTC days): your tries, your best solve and how many have solved it. The hint shows after two tries. |
| `careerDailyPuzzlePlay` | - | `CareerMatch` | session | Try today's puzzle. The board ranks first solves by tries, then seconds; the first solve of the day pays. |
| `careerPuzzleFilePlay` | `puzzle: Puzzle` | `CareerMatch` | session | Play a puzzle from a file (one exported from a practice game, or written by hand). It pays nothing. |
| `careerMinedPuzzles` | - | `CareerMinedPuzzle[]` | session | Puzzles mined from AI games (lethal the AI found), deepest first, with your stars. |
| `careerMinedPuzzlePlay` | `puzzleId: string` | `CareerMatch` | session | Try a mined puzzle. Stars go by tries, like the book's. |
| `careerHordes` | - | `CareerHorde[]` | session | The hordes: decks that play themselves. Live through a set number of their turns with a deck from the meta library. Your best (turns lived, life left) for each. |
| `careerHordePlay` | `hordeId: string`, `deckFile: string` | `CareerMatch` | session | Face a horde with a deck of the meta library (deckFile from metaLibrary). The first time you live through it pays. |
| `metaLibrary` | - | `MetaFormat[]` | session | The meta library: each format's school decks with their tier (the act they're met in) and the snapshot date. Free play, hordes and the weekly challenges draw from it. |
| `careerChallenge` | - | `CareerChallenge` | session | This week's challenge (the same fixed deck and opponent for everyone), your best result and the board. |
| `careerChallengePlay` | - | `CareerMatch` | session | Play this week's challenge. A win counts on the board by fewest turns, then most life left. |
| `careerGauntlet` | - | `CareerGauntletState` | session | The gauntlet: the entry price, rewards by wins, the run in progress (with its deck, bosses and any pick waiting) and recent runs. |
| `careerGauntletStart` | `options?: StartOptions` | `CareerRun` | session | Pay the entry and start a run: it offers starting packs to pick from. Options (all optional): mode (map, bossrush or endless), variant (jumpstart, pauper, singleton, cube or draft), ascension (0 up to the highest open), seed (a shared seed like K7Q2-9F3M), cubeId (one of getDraftCubes) or cubeCards (card names of a deck used as the cube), setCode (the set to draft). |
| `careerGauntletMove` | `nodeId: string` | `CareerRun` | session | Move to a node of the run's map that the current node links to: a fight waits to be played, a shop, rest or event makes its offer. |
| `careerGauntletPick` | `optionIds: unknown[]` | `CareerRun` | session | Take the waiting offer: as many starting packs as it asks for, or one bundle after a win. |
| `careerGauntletPlay` | - | `CareerMatch` | session | Play the run's next boss with the run's deck. |
| `careerGauntletAbandon` | - | `CareerRun` | session | Give up the run in progress. It pays nothing. |
| `careerLimited` | - | `CareerLimitedState` | session | Solo sealed and draft: prices, rewards by wins, the run in progress and recent runs. |
| `careerLimitedStart` | `kind: string`, `setCode: string`, `shape?: string`, `cubeCards?: string[]` | `CareerLimitedRun` | session | Pay the entry and start a sealed or draft run of a set the shop sells. Sealed opens six packs at once; a draft opens three per seat with seven bots. Every card opened joins the collection. Shape: classic (3 wins or 2 losses, the default), arena (7 wins or 3 losses) or league (5 matches, a pack joins the pool after each). Kind cube drafts a server cube (setCode is its name) or the card names in cubeCards (a deck sent as the cube); a cube's cards are loaners and don't join the collection. |
| `careerLimitedPick` | `index: number` | `CareerLimitedRun` | session | Draft: take the card at this position of the pack in front of you. |
| `careerLimitedCoach` | - | `CareerLimitedCoach` | session | The pool coach: every colour pair of the run's pool ranked by its ratings, removal, bombs and curve, each with its reasons and a 40-card build to start from. |
| `careerLimitedDeck` | `deck: DeckCardLists` | `CareerLimitedRun` | session | Submit the run's deck: at least 40 cards from the pool, plus any basic lands. |
| `careerLimitedPlay` | - | `CareerMatch` | session | Play the run's next game. The run ends at three wins or two losses. |
| `careerLimitedAbandon` | - | `CareerLimitedRun` | session | Give up the run in progress. Cards already opened stay in the collection. |
| `soloSaves` | - | `SoloSave[]` | session | Your saved games against the computer, newest first. Every solo game saves itself at the start of each turn; one still running says so (rejoin it from your open games). |
| `soloResume` | `saveId: string` | `SoloTable` | session | Carry on a saved game from the start of the turn it was saved in, at a new table with the same opponents and decks. A Career game still pays when it ends. |
| `soloDelete` | `saveId: string` | `boolean` | session | Throw a saved game away. |
| `soloSaveAndLeave` | `gameId: UUID`, `sessionId: string` | `boolean` | session | Free play against the computer: keep the game as saved at the start of this turn, and leave it. Resume it later from your saved games. |
| `soloFork` | `gameId: UUID`, `sessionId: string` | `SoloTable` | session | A game against the computer: start this turn again at a new table (the same cards in the same places, the same draws to come), and leave this game. Free play: as often as you like. Career: once a game outside puzzles and trials, and the game then pays half (a take-back). |
| `soloTurns` | `gameId: UUID` | `SoloTurn[]` | session | The turns of one of your games against the computer that can be played again (a what-if line), oldest first. Every turn of your latest games is kept, running or over. |
| `soloWhatIf` | `gameId: UUID`, `turn: number`, `sessionId: string` | `SoloTable` | session | Play one of your games against the computer again from the start of one of its turns, at a new table: a what-if line, always free play. A running game you're in is kept as a saved game first, untouched, and left; a Career game can be tried again this way once it's over. |
| `soloAssists` | `gameId: UUID`, `sessionId: string` | `Assists` | session | What you may still ask for in this game against the computer: hints (the AI's advice) and taking back the turn. Unlimited in free play; in Career three hints and one take-back a game, outside puzzles and trials, and any of them halves the game's pay. |
| `gameAdvise` | `gameId: UUID`, `sessionId: string`, `skill?: number` | `Advice` | session | What the AI would do in your place right now, without doing it: what to play, how to attack or block. Only in a game against the computer; ask while the game waits for you. In Career it's a hint: three a game, none in puzzles and trials, and the game then pays half. |
| `gamePuzzleExport` | `gameId: UUID`, `sessionId: string`, `name?: string`, `text?: string`, `objective?: string` | `PuzzleExport` | session | The board of a game against the computer as a puzzle (set it up with the practice tools first): both sides' life, hands, permanents (tapped or not), graveyards and the top of each library. Objective win (this turn, the default) or survive (their turn). What a card name can't bring back, like tokens and counters, is listed in left. Ask while the game waits for you. |
| `soloBatchSubmit` | `spec: JobSpec`, `kind?: string` | `BatchJob` | session | Queue AI-against-AI games between two decks (1 to 500, each seeded: game n plays seed + n). The server plays a few at a time; ask for the job to see the score. |
| `soloBatchJobs` | - | `BatchJob[]` | session | Your batches, newest first, with their scores so far. |
| `soloBatchJob` | `jobId: string` | `BatchJob` | session | One of your batches with every game's seed, winner and length. |
| `soloBatchCancel` | `jobId: string` | `boolean` | session | Stop one of your batches; games already played keep their score. |
| `leaderboard` | `mode: string`, `board: string`, `scope?: string`, `names?: unknown[]` | `Leaderboard` | session | A mode's leaderboard (weekly-challenge: board is the week, like 2026-W42; puzzle: board is the puzzle id), best first, with your line. Scope server (everyone) or friends (the names given, and you). |
| `adminBatchJobs` | - | `BatchStatus` | session | Staff (admin): the batch runner's settings, games running now and the latest jobs of every player. |
| `adminBatchCancel` | `jobId: string` | `boolean` | session | Staff (admin): stop anyone's batch. |
| `adminContentPacks` | - | `ContentPackView[]` | session | Staff (admin): the content packs on the server, each loaded or with the problems that kept it out. |
| `adminContentPackUpload` | `zipBase64: string`, `reason?: string` | `ContentPackView` | session | Staff (owner): add or replace a content pack from a zip (pack.json and decks/*.dck, up to 4 MB). It's checked as it loads; one with problems stays out and says why. |
| `adminContentPackRemove` | `packId: string`, `reason?: string` | `ContentPackView[]` | session | Staff (owner): remove a content pack. Progress players made in it stays, but its content goes. |
| `adminContentPackReload` | - | `ContentPackView[]` | session | Staff (admin): read the content pack folder again (after copying a pack in by hand). |
| `metaDeck` | `file: string` | `DeckCardLists` | session | A deck from the meta library (see metaLibrary), for the computer to pilot in free play. |
| `ghostShare` | `name?: string`, `deck: DeckCardLists` | `GhostDeck` | session | Share a deck as a ghost under a short code: anyone with the code can play against it, piloted by the computer, and each result reaches your inbox. |
| `ghostMine` | - | `GhostDeck[]` | session | The decks you share as ghosts, newest first, with their records. |
| `ghostUnshare` | `code: string` | `boolean` | session | Stop sharing a ghost deck. |
| `ghostDeck` | `code: string` | `GhostOpponent` | session | The ghost deck shared under a code, to seat the computer with. |
| `soloTableMark` | `tableId: UUID`, `ghostCode?: string`, `adaptiveFormat?: string` | `boolean` | session | Before starting a table of yours against the computer: play it against a ghost (its result goes to the deck's owner), or with adaptive skill (its result moves your hidden rating for the format). |
| `soloAdaptive` | `format: string` | `AdaptiveView` | session | Your adaptive skill for a format ("career", "constructed", "commander"...): the skill the next game plays at and how it's going, as a sentence. |
| `inbox` | - | `InboxNote[]` | session | Your latest notes, newest first: how your ghost decks did. |
| `inboxSeen` | - | `boolean` | session | Mark every note in your inbox seen. |
<!-- END GENERATED: methods -->

## Server events

Generated from `ClientCallbackMethod`. Do not edit the table by hand.

<!-- BEGIN GENERATED: callbacks -->
| Method | Data | Delivery | Description |
|--------|------|----------|-------------|
| `CHATMESSAGE` | `ChatMessage` | MESSAGE (may be dropped when outdated) | Message for the player or a log. |
| `SHOW_USERMESSAGE` | `string[]` | MESSAGE (may be dropped when outdated) | Message for the player or a log. |
| `SERVER_MESSAGE` | `ChatMessage` | MESSAGE (may be dropped when outdated) | Message for the player or a log. |
| `JOINED_TABLE` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `START_TOURNAMENT` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `TOURNAMENT_INIT` | `TournamentView` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `TOURNAMENT_UPDATE` | `TournamentView` | UPDATE (may be dropped when outdated) | State snapshot; newer ones replace older ones. |
| `TOURNAMENT_OVER` | `string` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `START_DRAFT` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `SIDEBOARD` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `CONSTRUCT` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `DRAFT_OVER` | `null` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `DRAFT_INIT` | `DraftClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `DRAFT_PICK` | `DraftClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `DRAFT_UPDATE` | `DraftClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `SHOW_TOURNAMENT` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `WATCHGAME` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `VIEW_LIMITED_DECK` | `TableClientMessage` | MESSAGE (may be dropped when outdated) | Message for the player or a log. |
| `VIEW_SIDEBOARD` | `TableClientMessage` | MESSAGE (may be dropped when outdated) | Message for the player or a log. |
| `USER_REQUEST_DIALOG` | `UserRequestMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_REDRAW_GUI` | `null` | CLIENT_SIDE_EVENT (may be dropped when outdated) | Not sent by the server. |
| `START_GAME` | `TableClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `GAME_INIT` | `GameView` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `GAME_UPDATE_AND_INFORM` | `GameClientMessage` | UPDATE (may be dropped when outdated) | State snapshot; newer ones replace older ones. |
| `GAME_INFORM_PERSONAL` | `GameClientMessage` | MESSAGE (may be dropped when outdated) | Message for the player or a log. |
| `GAME_ERROR` | `string` | MESSAGE (may be dropped when outdated) | Message for the player or a log. |
| `GAME_UPDATE` | `GameView` | UPDATE (may be dropped when outdated) | State snapshot; newer ones replace older ones. |
| `GAME_TARGET` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_CHOOSE_ABILITY` | `AbilityPickerView` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_CHOOSE_PILE` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_CHOOSE_CHOICE` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_ASK` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_SELECT` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_PLAY_MANA` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_PLAY_XMANA` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_GET_AMOUNT` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_GET_MULTI_AMOUNT` | `GameClientMessage` | DIALOG (ordered) | Prompt: the player must answer with a sendPlayer* call. |
| `GAME_OVER` | `GameClientMessage` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `END_GAME_INFO` | `GameEndView` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `REPLAY_GAME` | `null` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `REPLAY_INIT` | `GameView` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
| `REPLAY_UPDATE` | `GameView` | UPDATE (may be dropped when outdated) | State snapshot; newer ones replace older ones. |
| `REPLAY_DONE` | `string` | TABLE_CHANGE (ordered) | Lifecycle event: open, switch or close a screen. |
<!-- END GENERATED: callbacks -->

### Pre-game prompt markers

The bridge names the questions asked before the first turn, so clients don't have to recognize them by their text.
The marker is added to the JSON prompt only (the desktop client is unaffected), as `data.options.webPrompt`:

| Marker | Event | Question |
|--------|-------|----------|
| `mulligan` | `GAME_ASK` | Keep or mulligan the opening hand (`true` = mulligan). |
| `mulliganBottom` | `GAME_TARGET` | London mulligan: choose a card from hand to put on the bottom of the library. |
| `startingPlayer` | `GAME_TARGET` | Choose the player who takes the first turn. |

Markers are only set while `gameView.step` is absent (before the first turn). Clients should keep a fallback for
servers without the bridge marker.

## Deck import

`deckImportFromUrl(url)` reads a deck from a deck website link and returns an `ImportedDeck`: the site's card names
with set codes and, where the site has them, collector numbers, split into `main`, `side`, `commanders` and
`companion`, plus name, author, format (as an XMage deck type), cover card and when the deck last changed on the site.
The web client matches the cards against the card database itself (`lookupCards`), so a site's spelling never
decides what the server accepts.

The server reads Archidekt, TCGplayer Infinite, MTGTop8, Scryfall and ManaBox (`mage.server.websocket.service.deckimport`,
one `DeckSource` per site). Moxfield, MTGGoldfish, AetherHub, TappedOut and Deckstats block automated reads; the client
handles them by copy and paste, or with its "Send to Playmat" bookmarklet.

Rules the server keeps:

- It fetches only from the sites' own hosts, over https on port 443, following redirects only to those hosts. GET only, no cookies, an 8 second deadline and a 2 MB cap.
- Ten imports a minute per session. Results are cached for five minutes, and cached reads don't count.
- A site that fails three times in a row in a way that means it changed or blocks the server (`site_changed`, `blocked`) is reported as `degraded` by `deckImportSources` for 30 minutes; the client sends players to copy and paste instead.
- The log keeps per-site success and failure counts only, never links, deck contents or user names.

Failures come back as error `-32005` with `data: {"reason": ...}`:

| Reason | Meaning |
|---|---|
| `private` | The deck exists but isn't shared. |
| `not_found` | No deck at that link. |
| `blocked` | The site turned the server away (bot protection). |
| `rate_limited` | Too many imports from this session, or the site asked us to slow down. |
| `site_changed` | The site answered in a shape the server doesn't read. |
| `unsupported` | Not a deck link the server reads. |
| `too_large` | More than 2 MB. |
| `timeout` | The site didn't answer in time. |
| `unavailable` | The site is down or unreachable. |

Recorded responses from every site live in `Mage.Server/src/test/resources/deckimport` and back the unit tests.
To find out whether a site changed, run the live canary weekly (it imports one known public deck per site over
the network, and is skipped in normal builds):

```
mvn -pl Mage.Server test -Dtest=DeckImportLiveCanaryTest -Dxmage.deckImportCanary=true
```

## PlayerAction Enum Values

The `sendPlayerAction` method accepts a `PlayerAction` enum value as a string. Common values include:

### Priority Control
- `PASS_PRIORITY_UNTIL_MY_NEXT_TURN`
- `PASS_PRIORITY_UNTIL_TURN_END_STEP`
- `PASS_PRIORITY_UNTIL_NEXT_MAIN_PHASE`
- `PASS_PRIORITY_UNTIL_NEXT_TURN`
- `PASS_PRIORITY_UNTIL_NEXT_TURN_SKIP_STACK`
- `PASS_PRIORITY_UNTIL_STACK_RESOLVED`
- `PASS_PRIORITY_UNTIL_END_STEP_BEFORE_MY_NEXT_TURN`
- `PASS_PRIORITY_CANCEL_ALL_ACTIONS`
- `HOLD_PRIORITY`
- `UNHOLD_PRIORITY`

### Game Actions
- `UNDO`
- `CONCEDE`
- `ROLLBACK_TURNS`

### Mana Pool
- `MANA_AUTO_PAYMENT_ON`
- `MANA_AUTO_PAYMENT_OFF`
- `MANA_AUTO_PAYMENT_RESTRICTED_ON`
- `MANA_AUTO_PAYMENT_RESTRICTED_OFF`
- `USE_FIRST_MANA_ABILITY_ON`
- `USE_FIRST_MANA_ABILITY_OFF`

### Trigger Ordering
- `TRIGGER_AUTO_ORDER_ABILITY_FIRST`
- `TRIGGER_AUTO_ORDER_ABILITY_LAST`
- `TRIGGER_AUTO_ORDER_NAME_FIRST`
- `TRIGGER_AUTO_ORDER_NAME_LAST`
- `TRIGGER_AUTO_ORDER_RESET_ALL`

### Permissions
- `REQUEST_PERMISSION_TO_SEE_HAND_CARDS`
- `ADD_PERMISSION_TO_SEE_HAND_CARDS`
- `REVOKE_PERMISSIONS_TO_SEE_HAND_CARDS`
- `PERMISSION_REQUESTS_ALLOWED_ON`
- `PERMISSION_REQUESTS_ALLOWED_OFF`
- `REQUEST_PERMISSION_TO_ROLLBACK_TURN`
- `ADD_PERMISSION_TO_ROLLBACK_TURN`
- `DENY_PERMISSION_TO_ROLLBACK_TURN`

### Auto-Answer
- `REQUEST_AUTO_ANSWER_ID_YES`
- `REQUEST_AUTO_ANSWER_ID_NO`
- `REQUEST_AUTO_ANSWER_TEXT_YES`
- `REQUEST_AUTO_ANSWER_TEXT_NO`
- `REQUEST_AUTO_ANSWER_RESET_ALL`

### Client Actions
- `CLIENT_CONCEDE_GAME`
- `CLIENT_CONCEDE_MATCH`
- `CLIENT_QUIT_TOURNAMENT`
- `CLIENT_QUIT_DRAFT_TOURNAMENT`
- `CLIENT_STOP_WATCHING`
- `CLIENT_DISCONNECT_FULL`
- `CLIENT_DISCONNECT_KEEP_GAMES`
- `CLIENT_EXIT_FULL`
- `CLIENT_EXIT_KEEP_GAMES`
- `CLIENT_REMOVE_TABLE`
- `CLIENT_RECONNECT`
- `CLIENT_REPLAY_ACTION`
- `VIEW_LIMITED_DECK`
- `VIEW_SIDEBOARD`
- `TOGGLE_RECORD_MACRO`

## Example Flows

### Login Flow

```json
// 1. Connect user
{"jsonrpc": "2.0", "method": "connectUser", "params": ["myuser", "mypass", "any-id", "", "", ""], "id": 1}
// Response: {"jsonrpc": "2.0", "result": true, "id": 1}

// 2. Remember the restore token for a later reconnect (pass it as connectUser's 4th param)
{"jsonrpc": "2.0", "method": "sessionGetRestoreToken", "params": [""], "id": 4}
// Response: {"jsonrpc": "2.0", "result": "restore-token", "id": 4}

// 3. Set user data
{"method": "connectSetUserData", "params": ["myuser", "session-uuid", {"groupId": 0, "avatarId": 51, "confirmEmptyManaPool": true, "userSkipPrioritySteps": {...}, "flagName": "world.png"}, "1.0", ""], "id": 2}
// Response: {"jsonrpc": "2.0", "result": true, "id": 2}

// 4. Get main room ID
{"method": "serverGetMainRoomId", "params": [], "id": 3}
// Response: {"jsonrpc": "2.0", "result": "room-uuid-string", "id": 3}
```

### Create and Start Game

```json
// 1. Create table
{"method": "roomCreateTable", "params": ["session-uuid", "room-uuid", {"name": "My Game", "gameType": "Two Player Duel", "deckType": "Constructed - Standard", "winsNeeded": 1}], "id": 1}
// Response: {"jsonrpc": "2.0", "result": {...TableView...}, "id": 1}

// 2. Join the table
{"method": "roomJoinTable", "params": ["session-uuid", "room-uuid", "table-uuid", "MyPlayer", "Human", 5, {"name": "MyDeck", "cards": [...], "sideboard": [...]}, ""], "id": 2}
// Response: {"jsonrpc": "2.0", "result": true, "id": 2}

// 3. Start the match (when all players joined)
{"method": "matchStart", "params": ["session-uuid", "room-uuid", "table-uuid"], "id": 3}
// Response: {"jsonrpc": "2.0", "result": true, "id": 3}
```

### Game Actions

```json
// Pass priority until next main phase (F6 equivalent)
{"method": "sendPlayerAction", "params": ["PASS_PRIORITY_UNTIL_NEXT_MAIN_PHASE", "game-uuid", "session-uuid", null], "id": 1}

// Select a target
{"method": "sendPlayerUUID", "params": ["game-uuid", "session-uuid", "target-permanent-uuid"], "id": 2}

// Respond Yes to a question
{"method": "sendPlayerBoolean", "params": ["game-uuid", "session-uuid", true], "id": 3}

// Choose an X value
{"method": "sendPlayerInteger", "params": ["game-uuid", "session-uuid", 5], "id": 4}
```
