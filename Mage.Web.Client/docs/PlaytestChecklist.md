# Playtest checklist

The first sessions with the private group. The goal is a ranked list of real friction, not a feature tour. Run it on the public server (`https://playmat.deimos.studio`), or a LAN server (`https://multivac.local:8443`, see `ops/web/README.md`), with two to four players.

## Before the session

- [ ] Server is up: `docker compose -f ops/web/docker-compose.yml ps` shows `xmage` and `web` healthy.
- [ ] Each player opened the site once and accepted the certificate warning (or installed Caddy's root certificate).
- [ ] Desktop browsers only, window at least 1280×720. Note each player's browser and screen size.
- [ ] One person takes notes in the bug template below. Screenshots go in a shared folder named by time (`21-14 blockers.png`).
- [ ] Server log is reachable for timestamps: `docker compose -f ops/web/docker-compose.yml logs -f xmage`.

## Scenarios

Time each one. Don't explain the UI first: watching someone hesitate is the data.

### 1. First game against the AI (each player, alone)

- [ ] Sign in with a name and reach Play.
- [ ] Pick a starter deck, play against the AI, finish the game (win or concede).
- [ ] Record: clicks from sign-in to the first mulligan screen; anything they looked for and didn't find.

### 2. Their own deck

- [ ] Import a deck they actually play (paste from Moxfield, Archidekt or MTGA export), fix anything flagged illegal, save it.
- [ ] Play it against the AI.
- [ ] Record: import formats that failed, cards that showed the wrong image or none.

### 3. Each other, best of three

- [ ] One player hosts a table (Against a friend), another joins it from Tables.
- [ ] Play a best-of-three with sideboarding between games.
- [ ] Mid-game: one player reloads the browser tab. Then one player turns Wi-Fi off for 20 seconds and back on.
- [ ] Record: whether both got back into the game and how long it took; whether sideboarding offered the right cards.

### 4. Combat and stack stress

Use decks with combat tricks, flash and triggers.

- [ ] Multiple blockers on one attacker (damage assignment).
- [ ] An instant in response to a spell; a triggered ability with a choice.
- [ ] A tutor, scry or "look at the top cards" effect.
- [ ] X spells and mana from more than one colour source.
- [ ] Record: any moment someone didn't know whose priority it was or what the game was waiting for.

### 5. Watching

- [ ] A third player watches scenario 3 from Tables, then leaves.
- [ ] Record: what the watcher could see, and whether leaving left them stuck as a watcher.

### 6. Commander pod

- [ ] Build or copy a commander deck: commander in the command zone strip, 100/100, "Commander's colors" filter on.
- [ ] Play vs AI with the Commander pod set to 4 players (or host a Commander Free For All with AI seats).
- [ ] Cast the commander from the command zone, let it die, recast it: the pile shows the +2 tax.
- [ ] Attack with several opponents at the table: the picker names who each creature can hit; seats show commander damage taken.
- [ ] Record: whether the four-seat layout stays readable, and how long AI turns feel.

### 7. Draft (if time allows)

- [ ] Host a booster draft with AI seats, draft, build, play one round.
- [ ] Record: pick timer pressure, how the pool reads while building.

## After the session

- [ ] Five-minute round: each player names the single worst moment and the single best one.
- [ ] Rank the notes: blocks play > wrong result > confusing > cosmetic.
- [ ] Add each item to the backlog tracker (https://claude.ai/artifact/DAHwcrRiJseP94iUjjhR5A) under M2 with the playtest date in the evidence field.
- [ ] Back up anything interesting from the server log before it rotates.

## Bug template

```
When:      21:14 (server log time)
Who:       player, browser, screen size
Doing:     scenario + step
Expected:  what they thought would happen
Happened:  what happened instead
Severity:  blocks play | wrong result | confusing | cosmetic
Evidence:  screenshot name, log lines
```
