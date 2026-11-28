# HumanPlayer dialogs: the question sequence a decision log replays

What Java's `HumanPlayer` asks, in what order, with a client's default settings
(`UserData.getDefaultUserDataView()`), so the Rust engine asks the same questions and consumes the same answers.

Paths are shortened:
- `HP` = `Mage.Server.Plugins/Mage.Player.Human/src/mage/player/human/HumanPlayer.java`
- `PI` = `Mage/src/main/java/mage/players/PlayerImpl.java`
- `GI` = `Mage/src/main/java/mage/game/GameImpl.java`
- `DL` = `Mage/src/main/java/mage/game/trace/DecisionLog.java`

Checked against two recorded shadow games (`ShadowGamesTest`, pioneer decks, 8 turns, 117 and 104 answers). Every
sequence quoted as "seen" below comes from those recordings.

## 0. The mechanics of one question

Each question follows the same steps: `prepareForResponse` (HP:295), then `game.fireXxxEvent` (the PlayerQueryEvent),
then `waitForResponse` (HP:321).

- `prepareForResponse` **sets `state.priorityPlayerId` to the asked player** whenever it isn't null (HP:301-311). That
  is a state change: the fingerprint's `priority` seat follows the last player asked, so a trigger question asked
  of the other player during `checkStateAndTriggered` leaves `priority` on that player at the next `PRIORITY`
  trace. Rust must do the same.
- `waitForResponse`: when replaying, `replayResponse` (HP:442) takes `decisions.next(game, id)` and fills
  `response` with it. When recording, the raw answer is recorded after the async concede/cheat loop, and only when
  the game hasn't ended (HP:432-435). Each question records exactly one answer, **including answers the dialog then
  ignores** (an unplayable card, a tapped land). Rust must ask again and consume the next answer, exactly as Java
  re-asks.
- The UUID is recorded raw, before `getFixedResponseUUID` maps an MDFC half to its main card (HP:1555).
- An answer tuple is `{seat, target?, bool?, integer?, string?, manaType?+manaPlayer?}`. A client sets one field.
  Each dialog tests the fields in a fixed order (given per flow below), so Rust must test them in that same order.
- **Not recorded:** `sendPlayerAction` (F-keys and "pass until…", HOLD_PRIORITY, trigger auto-order, auto-answer
  yes/no, concede). An F-key sets a skip flag and also sends `skip()` → `response.setInteger(0)` (HP:2873). Only the
  `int 0` is recorded. A replay therefore assumes that no skip flag (`passedTurn`, `passedUntilEndOfTurn`,
  `passedUntilNextMain`, `passedAllTurns`, `passedUntilStackResolved`, `passedUntilEndStepBeforeMyTurn`,
  `passedTurnSkipStack`) is ever set. The only exception is `PASS_PRIORITY_CANCEL_ALL_ACTIONS`, which the server
  sends itself and which clears them. **This whole spec assumes all skip flags are false.** A log from a client
  that used F2–F9 won't replay.
- `canCallFeedback` = not in a check-playable state and not a simulation. A dialog that fails this asks nothing and
  returns a default.

Event → QueryType (GI:3405-3566, `PlayerQueryEvent`):

| Fired by | QueryType |
|---|---|
| `firePriorityEvent`, `fireSelectEvent` | SELECT |
| `fireAskPlayerEvent` | ASK |
| `fireSelectTargetEvent` (targets/cards/perms) | PICK_TARGET |
| `fireSelectTargetTriggeredAbilityEvent` | PICK_ABILITY |
| `fireGetChoiceEvent` (abilities) | CHOOSE_ABILITY |
| `fireGetModeEvent` | CHOOSE_MODE |
| `fireChooseChoiceEvent` | CHOOSE_CHOICE |
| `firePlayManaEvent` | PLAY_MANA |
| `fireGetAmountEvent` (announceX too) | AMOUNT |
| `fireGetMultiAmountEvent` | MULTI_AMOUNT (message null; title/header in options) |

Every message gets `getControllingPlayerHint` appended: `" (as <name>)"` when another player controls the turn,
empty otherwise. Many messages embed `getLogName()`, which contains HTML and the object's UUID (for example
`<font color='#90EE90' object_id='…'>Grizzly Bears</font> [97e]`), so Rust can't reproduce the message text byte for
byte. Answers never depend on the text.

### Pre-game (seen)
1. Starting player: `choosingPlayer.choose(TargetPlayer "starting player")` (GI:1561-1586) → PICK_TARGET
   `Select a starting player`, answered `P<seat>`. Source is null, so there is no auto-choice.
2. Mulligans, in turn order from the starting player (`Mulligan.java:44-80`): ASK
   `Mulligan <font color=#ffff00>down to 6 cards</font>?` (or `Mulligan <green>for free</green>, draw another N cards?`),
   with buttons Mulligan/Keep. `true` = mulligan, `false` = keep. A non-boolean answer re-asks (HP:481-513). London
   bottoming: PICK_TARGET for `TargetCardInHand "card (N more) to put on the bottom of your library"`.

## 1. Priority — `HP.priority` (HP:1302) inside `GI.playPriority` (GI:2051)

### The engine loop
- Each step with `hasPriority` runs `playPriority(active)` after `beginStep` (`Step.java:70`, `Phase.playStep`).
  UNTAP has no priority. CLEANUP gives priority only when 514.3a applies (`EndPhase.playStep`:
  `checkStateAndTriggered()` true). DECLARE_BLOCKERS and both damage steps are skipped when there are no attackers;
  FIRST_COMBAT_DAMAGE is also skipped without first or double strike. **DECLARE_ATTACKERS is never skipped**, so it
  gives priority even when nobody attacked (seen every turn).
- Round: `resetPassed`, current = active (GI:2064). For each player in APNAP order:
  `while (!player.isPassed())` → `checkStateAndTriggered()` (trigger questions happen here, before the SELECT) →
  `trace(PRIORITY)` → `player.priority(game)` (GI:2078-2101). So **a player who acts gets priority again** and is
  asked again until they pass. Any successful activation (land, spell, ability, **including a mana ability**) calls
  `resetPassed()` for every player (PI:1762).
- When everyone has passed and the stack isn't empty, the top object resolves, then a new round starts with the
  **active player** (GI:2109-2118). When everyone has passed and the stack is empty, the step ends, and the mana
  pools empty in `Phase.postPriority`.

### Skip rules (with defaults, no skip flags)
`priority()` first runs the skip section (HP:1342-1462). With every flag false, only two rules can apply:

- **Empty stack: `checkPassStep`** (HP:1573) passes without a question when the step is **not** set in
  `SkipPrioritySteps` (yourTurn for the active player, opponentTurn otherwise; both have the same defaults).
  `isPhaseStepSet`: UPKEEP=false, DRAW=false, PRECOMBAT_MAIN=true, BEGIN_COMBAT=false, END_COMBAT=false,
  POSTCOMBAT_MAIN=true, END_TURN=false. **Every other step returns true (`default`)**, which covers DECLARE_ATTACKERS,
  DECLARE_BLOCKERS, FIRST_COMBAT_DAMAGE, COMBAT_DAMAGE and CLEANUP.

  | Step (empty stack) | Active | Non-active |
  |---|---|---|
  | UPKEEP, DRAW, BEGIN_COMBAT, END_COMBAT, END_TURN | auto-pass | auto-pass |
  | PRECOMBAT_MAIN, POSTCOMBAT_MAIN | **ask** | **ask** |
  | DECLARE_ATTACKERS (even with no attackers), DECLARE_BLOCKERS, FIRST_COMBAT_DAMAGE, COMBAT_DAMAGE | **ask** | **ask** |
  | CLEANUP (only on a 514.3a priority) | ask | ask |

- **Non-empty stack: always ask.** `passedUntilStackResolved` is false. The `stopOnStackNewObjects` and
  `dateLastAddedToStack` bookkeeping (HP:1444-1460) only matters when that flag is set.
- The "stop on declare attackers / blockers" quick stop (HP:1328-1340): in DECLARE_ATTACKERS, a defending player
  with at least one possible blocker sets `quickStop` and bypasses the skip section. With the defaults this changes
  nothing, because DECLARE_ATTACKERS asks anyway. `passPriorityCast` and `passPriorityActivation` are false, so
  the `justActivatedType` block (HP:1311-1323) does nothing.
- Every auto-pass goes through `passWithManaPoolCheck` (below), so an auto-passed step can still ask the
  floating-mana ASK.

### The question
SELECT, `Play spells and abilities` when `canPlaySorcery` (main phase, active player, empty stack), otherwise
`Play instants and activated abilities` (GI:3405-3419). The question loop is at HP:1466-1492.

Answers, tested in this order:
1. `bool != null || integer != null` → `passWithManaPoolCheck`. **true and false both mean pass**, and the integer
   is the F-key `0`. If the floating-mana question gets a "no", the loop `continue`s and fires SELECT again, without
   re-running the skip section.
2. `string == "special"` → `activateSpecialAction(game, null)` (HP:2396). If special actions exist, CHOOSE_ABILITY
   with **message = the player's name** (HP:2409). The answer is the action id, `X<i>`. Otherwise nothing happens.
3. A UUID → `getPlayableActivatedAbilities(object, zone)` (PI:1891 → `getPlayable(game, true, zone, false)`, which
   filters to the object's parts and keeps the getPlayable order). If the map isn't empty →
   `activateAbility(map, object)` (HP:2430) and `priority` returns **true even if the activation was then
   cancelled**. If it is empty (unaffordable, wrong timing, opponent's card, tapped land, summoning-sick creature,
   a player id, an unknown id) → returns false. In every case the player hasn't passed, so **the engine asks priority
   again** (and re-runs the skip section first). Face-down cards may show a look dialog instead.
4. Otherwise returns `manaType == null`. A mana-pool click at priority does nothing.

### `activateAbility(Map, object)` (HP:2430-2475): the ability picker
- If there is one ability and `suppressAbilityPicker` (HP:2481) holds, it activates directly with no question. The
  surrounding `if` (HP:2437-2445) is always true: it ORs three negations, one of which is always true. Suppressed
  when:
  - the main card is off the battlefield, is a `CardWithSpellOption` (adventure/omen…), and exactly one of its
    main/spell options is available. Otherwise an MDFC, split card or spell-option card off the battlefield
    **always shows the picker, even with one ability** (seen: MDFC land Needleverge Pathway → CHOOSE_ABILITY →
    `H0.7a2`).
  - `PlayLandAbility`.
  - no alternate-mana cast is registered and `getManaCostsToPay().manaValue() > 0`. That covers ordinary spells and
    activated abilities with mana in their cost.
  - `isManaActivatedAbility()`.

  Not suppressed, so **a picker with one option**: a spell with mana value 0 (e.g. Ornithopter), and a non-mana
  activated ability whose cost has no mana (`{T}: …`, `Sacrifice …: …`).
- `useFirstManaAbility` is false by default (HP:2449).
- Otherwise: CHOOSE_ABILITY `Choose spell or ability to play<br><logName>`. The answer is the ability id, e.g.
  `B7a3` or `H0.7a2`. If it isn't in the map, nothing happens and control returns to the priority loop.
- `HP.activateAbility(ability)` calls `getManaPool().setStock()` before `PI.activateAbility` (HP:2423). This matters
  for mana (section 3).

### `passWithManaPoolCheck` (HP:3035)
This asks only if `userData.confirmEmptyManaPool` **and the stack is empty** and the pool has mana that
`canLostManaOnEmpty`. Then it calls `chooseUse` → ASK
`<font color='#D2691E'>You still have mana in your mana pool and it will be lost. Pass anyway?</font>`
(Yes/No, no source). `true` → `pass()`. `false` → `PASS_PRIORITY_CANCEL_ALL_ACTIONS` and no pass. Non-boolean answers
re-ask, per `chooseUse`. Floating mana with a non-empty stack is never asked about.

## 2. Playing a land
SELECT → `H<seat>.<i>` → the playable map is `{PlayLandAbility}`, which is suppressed → `PI.activateAbility` →
`playLand` (PI:1445). No question of its own. `chooseApprovingObject` (PI:1537) asks a CHOOSE_CHOICE
`Choose the permitting object` only when more than one permission exists. ETB replacements ask their own questions
(seen: shock land → ASK `pay 2 life? (otherwise Sacred Foundry becomes tapped)` straight after the SELECT). A second
land in the same turn, or a land at the wrong time, isn't playable, so priority is asked again. An MDFC land shows
the picker (section 1).

## 3. Casting a spell: Grizzly Bears, then Shock

Order (`PI.cast` PI:1358 → `AbilityImpl.activate`, `Mage/src/main/java/mage/abilities/AbilityImpl.java:289`):
1. The SELECT answer `H…` → picker or direct (section 1). `chooseApprovingObject`.
2. `card.cast`: **the card moves to the stack (S0) before any further question**. Later answers name it `S0…`,
   e.g. the mode answer `S0sm2` (seen).
3. Alternative/additional costs (`activateAlternateOrAdditionalCosts`). CHOOSE_CHOICE
   `You may choose an alternative cost` appears only if one exists. Kicker and similar costs ask their own ASK.
4. `handleChooseCostTargets`. Then **X**: `announceX` (HP:1804) → AMOUNT
   `Announce the value for {X} (source: <logName>)`, with min/max. No question when `min >= max`. An answer that is
   null or out of range re-asks. Then Phyrexian costs (ASK per symbol, `Pay 2 life instead of …?`).
5. Modes: `chooseMode` (HP:2606). No question for a single mode. Otherwise CHOOSE_MODE
   `Choose mode (selected k of N, min m)<br><logName>`. The answer is a mode id (`S0sm<k>`). The Done/Cancel options are
   fixed UUIDs (`Modes.CHOOSE_OPTION_DONE_ID`, `…CANCEL_ID`) that DecisionLog can't name (see 11). For a
   non-triggered ability, any non-mode answer returns null and cancels the cast.
6. Targets: `Targets.chooseTargets(…, canCancel = activated && human)` sets `required=false` (`Targets.java:115`) →
   `TargetImpl.chooseTarget` → auto-choice (section 10) or `HP.chooseTarget` (HP:869), one PICK_TARGET per target.
   Shock: `Select any target` (`TargetImpl.getMessage` + `addArticle`; "any …" gets no article). Both players are
   legal targets, so it is never auto-chosen.
   - A UUID in possibleTargets → added; when complete (1 of 1), returns.
   - A UUID already chosen → removed, ask again.
   - Another UUID → ignored, ask again.
   - A boolean (Done/Cancel): with nothing chosen and min > 0 → returns false → `activate` fails → **cast cancelled,
     `restoreState(bookmark)`**, so the card goes back to hand and the lands untap. For an "up to one" target, a
     boolean means none and the cast goes on (seen with a trigger: `Select up to one creature you don't control with
     flying` → `bool true`).
   - **Nothing to choose: no question.** When `possibleTargets` is empty and nothing is chosen yet, `HP.chooseTarget`
     returns false without a PICK_TARGET (since #265). With min > 0 the cast or trigger fails as before; an "up to"
     target goes on with none, exactly as the boolean above did. Effects that call `chooseTarget` behave the same:
     Relic of Progenitus on an empty graveyard (which also checks `canChoose` first) and `ChooseCreatureEffect` with
     no other creature ask nothing. Only `HP.chooseTarget(outcome, target, …)` changed: `HP.choose` (sacrifice,
     discard, Aether Vial, Lively Dirge, BlightCost), `HP.chooseTarget(outcome, cards, …)` and `HP.choose(cards)`
     still show a prompt with nothing selectable, and `chooseTargetAmount` (divided "up to" targets) still asks.
     Effects that skip the call themselves still skip it (SearchLibraryGraveyardPutInHandEffect asks nothing for an
     empty graveyard, but asks for a graveyard with no match). Shadow games recorded before #265 have an extra
     boolean answer at these points.
7. Cost modification, then **mana**: `ManaCostsImpl.pay`
   (`Mage/src/main/java/mage/abilities/costs/mana/ManaCostsImpl.java:132`).
   - `assignPayment` from the pool first.
   - Then loop `while (!isPaid)`: `playMana` → PLAY_MANA `Pay <unpaid>` + `<div style='font-size:11pt'><logName></div>`
     (`ManaUtil.addSpecialManaPayAbilities`, `Mage/src/main/java/mage/util/ManaUtil.java:440`; HP:1731-1768). After
     each handled answer, `assignPayment` again.
   - **One question per payment step** (seen: `Pay {G}{G}{G}` → `Pay {G}{G}` → `Pay {G}`).

   PLAY_MANA answers, in order (HP:1746-1762):
   - `bool` (either value) → cancel → `pay` false → cast rolled back.
   - A UUID → `playManaAbilities` (HP:1837): `getUseableManaAbilities(object, zone)` (activatable mana abilities only)
     → `ManaUtil.tryToAutoPay` (`ManaUtil.java:65`). If every ability is a basic or any-color mana ability, it keeps
     the one that fits the unpaid colored symbols; with no colored symbol to pay but generic owed, it keeps the first.
     If one is left, it is activated directly. If several are left (a dual land paying `{1}`, two colors both needed),
     CHOOSE_ABILITY `Choose spell or ability to play<br><logName>` asks for an ability id (`B<i>a<j>`); a wrong id does
     nothing. **No usable mana ability** (tapped, summoning sick, non-mana permanent) → nothing; the same PLAY_MANA is
     asked again (seen: `B4` twice, unchanged prompt). A mana ability that adds "any color" asks CHOOSE_CHOICE
     `Choose …`, unless `tryToAutoSelectAManaColor` finds a single useful color (HP:735-749).
   - `string "special"` → `activateSpecialAction(game, unpaid)`: convoke and other special mana actions, via
     CHOOSE_ABILITY with message = player name.
   - `manaType` + `manaPlayer` = self → `unlockManaType(type)` → the next `assignPayment` takes **one** mana of that
     type from the pool (`lockManaType` after each, `ManaPool.java:115-180`). A manaPlayer other than yourself is
     ignored.
   - **Floating mana is not used automatically when it was already in the pool before the spell started**
     (`manaPoolAutomaticRestricted=true`). `setStock()` freezes the pool at activation start. `ManaPool.pay` skips
     items whose `count == stock`, and pays nothing at all while no mana was added beyond stock (`ManaPool.java:121-
     167`). So tapping Llanowar Elves at priority and then casting Bears still asks PLAY_MANA `Pay {1}{G}`. Paying
     needs a `manaType GREEN` answer (or a land). Mana added **during** this payment (by tapping a source) is used
     automatically, along with any excess. Since every activation, mana abilities included, calls `setStock()`, mana
     floated by an earlier action is always "stock" for the next one.
8. Other costs (tap, sacrifice, discard; these ask their own PICK_TARGET). `SPELL_CAST`. Then `priority()` returns
   true and **the caster gets priority again** with a non-empty stack: SELECT `Play instants and activated
   abilities`. Then the opponent. Seen: cast → PLAY_MANA → caster SELECT → opponent SELECT → resolve → active SELECT.

Activated abilities (`playAbility`, PI:1604) follow the same `activate` order: the ability goes on the stack first,
then X, modes, targets, and mana.

## 4. Mana ability at priority (Llanowar Elves, nothing to pay)
SELECT → `B<i>` → the playable map holds the mana ability, which is suppressed
(`isManaActivatedAbility`) → `playManaAbility` (PI:1582) → G goes to the pool. **No question.** Then `resetPassed`, and
priority is asked again. Two mana abilities (a dual land, or Hive of the Eye Tyrant with its manland ability) →
CHOOSE_ABILITY `Choose spell or ability to play<br><logName>` (seen `B7` → `B7a3`). There is no `tryToAutoPay` at
priority (`currentlyUnpaidMana` is null), so an any-color ability asks CHOOSE_CHOICE for the color. Passing later
with that G floating and an empty stack asks the floating-mana ASK.

## 5. Declaring attackers — `HP.selectAttackers` (HP:1886)
- Called from `DeclareAttackersStep.beginStep` → `Combat.selectAttackers`
  (`Mage/src/main/java/mage/game/combat/Combat.java:261`), **before** the step's priority. The first time it is
  called only if `getAvailableAttackers()` is non-empty. The `do { … } while (!checkAttackRestrictions)` loop asks
  again on an illegal configuration.
- The skip at HP:1906-1918 needs skip flags, so with defaults it never fires.
- SELECT `Select attackers`. Options: `POSSIBLE_ATTACKERS`, plus `SPECIAL_BUTTON "All attack"` when any are available.
  Answers, in order:
  - `"special"` = **all attack**. The defender is the only defender if there is one, otherwise PICK_TARGET
    (`selectDefenderForAllAttack`). Every `filterCreatureForCombat` creature you control is declared. **Then it asks
    again** (seen: `special` then `bool true`).
  - `integer` or `bool` (either value) → `checkIfAttackersValid`. Valid → done. Invalid (forced attackers, "must
    attack") → informPlayer (not a question), then ask again.
  - A UUID: a non-attacking untapped creature (`filterCreatureForCombat`) → `selectDefender` → declared, or silently
    not declared if it can't attack (summoning sick). An attacking creature with an empty stack →
    `removeAttackerIfPossible`, which is a toggle. Anything else is ignored. In every case it asks again.
- Attack target (`selectDefender`, HP:2115): if the possible defenders (forced set, or `combat.getDefenders()` = the
  opponent plus their planeswalkers and battles) number exactly 1, there is **no question**. In a plain two-player
  game this is the opponent. Otherwise PICK_TARGET `Select a player, planeswalker, or battle to attack` (`TargetDefender`,
  source null, so no auto-choice; it is required, so a boolean asks again). Answer: `P<seat>` or `B<i>`.

## 6. Declaring blockers — `HP.selectBlockers` (HP:2156)
- Called from `DeclareBlockersStep.beginStep` → `Combat.selectBlockers`, once for each defending player. The
  `while(true)` loop asks again after the restriction and requirement checks fail (`Combat.java:689-735`).
- `possibleBlockersCount` = your untapped, non-attacking, phased-in creatures with free block capacity
  (`FilterCreatureForCombatBlock`). **It isn't checked against the actual attackers**, so a flyer attacking into
  Grizzly Bears still asks. If it is 0, there is no question (HP:2179). The skip-button branch needs flags.
- SELECT `Select blockers` (options `POSSIBLE_BLOCKERS` = the legal ones). Answers, in order:
  - `bool` (either value) or `integer` → done.
  - A UUID: a creature that matches the filter → `selectCombatGroup` (HP:2224). If it can block exactly one attacker,
    that attacker is **auto-chosen, with no question**. Otherwise PICK_TARGET `Select attacker to block` (required
    false). The answer `B<i>` declares the block, or removes it if the creature already blocks that attacker. A
    boolean does nothing. A creature already blocking at full capacity, with an empty stack → removes the block.
    Anything else is ignored.
  - Then it **asks again** (seen: `Select blockers` → `B8` → `Select blockers` → `bool true`).
- After the blocks, the defender's first priority comes in DECLARE_BLOCKERS. The active player is asked first (APNAP).

## 7. Combat damage assignment (`Mage/src/main/java/mage/game/combat/CombatGroup.java`)
Asked in `CombatDamageStep.beginStep`, before that step's priority. It calls `getMultiAmountWithIndividualConstraints`
(HP:2313). There is **no question** if there are no messages, or a single message with min == max, or every message
has min == max (for example 0 damage). Otherwise MULTI_AMOUNT (message null; options title/header). The answer is a
`string` of space-separated ints in message order (`MultiAmountType.parseAnswer`). Bad values → informPlayer
"Error, you must enter correct values." and ask again. **A boolean is ignored and asks again** (`canCancel` false).
- **Several blockers on one attacker, no trample** (CombatGroup:309-322): message per blocker
  `<logName>, P/T: p/t`, each 0..damage, sum == damage. Defaults are lethal, in blocker order. The attacker's
  controller is asked, or the defender for `defenderAssignsCombatDamage`.
  Title `Assign combat damage`, header `Assign combat damage among creatures blocking <name>, P/T: …`.
- **Trample** (CombatGroup:292-308): asked if `remainingDamage > 0` (damage exceeds the summed lethal) or there is
  more than one blocker. The sum ranges from the summed lethal to damage; the rest tramples over. One blocker with
  damage ≤ lethal → no question.
- **One blocker on several attackers** (`attackerDamage`, CombatGroup:456): same dialog for the blocker's controller.
- One attacker, one blocker, no trample → no question.
- Special: `DamageAsThoughNotBlockedAbility` asks ASK `Have <name> assign damage as though it weren't blocked?`
  (CombatGroup:176). Divided-damage abilities ask ASK at CombatGroup:859.

## 8. Cleanup discard (`CleanupStep.beginStep` → `PI.discardToMax`, PI:836)
- Only if hand > max (7). `discard(n)` → `getToDiscard` (PI:905) → `TargetDiscard(n, n)` → `TargetImpl.choose` →
  `HP.choose(outcome, target, source = null)` (HP:782).
- **No auto-choice** (source is null). One PICK_TARGET per pick: n = 1 → `Select a card to discard`; n > 1 →
  `Select card to discard (selected k of n, min n)`. Options carry `possibleTargets` = the hand.
- Answers:
  - A hand card (`H<seat>.<i>`) → added; once it reaches n, done.
  - A card already selected → removed.
  - Another UUID → ignored.
  - A boolean before n is reached → asks again (required).
- No priority follows in cleanup unless 514.3a applies.

## 9. Triggers
- Placement: `GI.checkTriggered` (GI:2693), in APNAP order starting with the active player. Non-stack triggers
  resolve first. **A player with one trigger is not asked about order.** With more than one,
  `chooseTriggeredAbility` (HP:1602) runs, and is called again for each remaining trigger until one is left.
- `autoOrderTrigger=true`: no question when every trigger has the same `getRule(sourceName)` text, the same effect
  count and the same target-pointer targets (identical copies). It returns the first. The trigger-order lists set by
  player actions start empty. Otherwise PICK_ABILITY `Pick triggered ability (goes to the stack first)`. The answer
  is the ability id (`T<seat>.<i>`); a source id isn't accepted, because `macroTriggeredSelectionFlag` is still true.
  An unmatched answer asks again.
- Each placed trigger then runs `PI.triggerAbility` → `activate` → targets (section 10). A triggered ability's target
  can't be cancelled: a boolean asks again while the target is required.
- "May" triggers ask **at resolution** (`TriggeredAbilityImpl.resolve:263`): ASK = `getRule(object.getLogName())`
  (e.g. `When … you may …`), buttons Yes/No. The auto-answer maps start empty. Effect-level "may" uses
  `chooseUse(…, message, …)` with card-specific text.
- `chooseUse` (HP:522): only a boolean ends it; anything else asks again.

## 10. Targets and autoTargetLevel = 1 (`TargetImpl.tryToAutoChoose`, `Mage/src/main/java/mage/target/TargetImpl.java:852`)
A target is auto-chosen, with **no question**, only if all of these hold:
- `source != null` (so cleanup discard, the attack target and the starting player are never auto-chosen)
- `min == max`
- the controller can respond (online)
- `possibleTargets.size() == min - alreadyChosen` (exactly as many legal choices as needed; already-chosen ones are
  removed first)
- not strict mode
- level > 0
- the rule text (lower case) has no `search`

For level 1 (not 2), the first possible id in iteration order that passes:
- skip ids already chosen;
- `possibleChooseId == abilityControllerId` (**reference** `==`) means yourself. Otherwise permanent → "own" if the
  controller `==` you; card → owner `==` you; spell → controller `==` you. **Anything else, which includes the
  opponent player, is skipped. So a sole player target is never auto-chosen** (except perhaps yourself).
- own things are skipped if the rule text has `discard`, `sacrifice`, `destroy` or `exile`.

The UUID comparisons are Java identity, which in practice matches equality (the ids come from the same objects).
Rust should use equality.

`TargetImpl.chooseTarget` tries the auto-choice before calling `HP.chooseTarget`, and `HP.chooseTarget` tries again.
Each auto-choice adds one target and loops.

Other dialogs:
- `choose(Choice)` (HP:719): CHOOSE_CHOICE. A non-empty `string` is accepted **without validation** (an unknown key
  leaves the choice unset but returns true). An empty string cancels if not required, and asks again if required.
- `getAmount` (HP:2279): same rules as announceX.
- `chooseReplacementEffect` (HP:613): no question for one effect, or when every option has the same text.
  `useSameSettingsForReplacementEffects` only matters for remembered `#` answers. CHOOSE_CHOICE answers are a `string`
  key (`#key` remembers it).
- `chooseTargetAmount` (HP:1133): PICK_TARGET with ` to divide N damage`, then MULTI_AMOUNT. No MULTI_AMOUNT for one
  target, or when targets == amount.

## 11. How DecisionLog names answers (DL)
- **Seat** = index in `game.getState().getPlayers().keySet()` (`Players` is a `LinkedHashMap`), in **the order
  players were added** (table seating), not turn order, and not starting player first. `StateFingerprint.seats`
  uses the same order. Recorded seat = the asked player (`getId()`).
- `ref(id)` = the first match, visited in this order (DL:`visit`):
  1. `P<s>` for each player
  2. for each seat: hand `H<s>.<i>`, then library `L<s>.<i>`, then graveyard `G<s>.<i>`
  3. battlefield `B<i>`
  4. stack `S<i>`
  5. exile `E<i>`
  6. command zone `M<i>`
  7. special actions `X<i>`
  8. pending triggers `T<s>.<i>` (`state.getTriggered(seatId)`, list order)

  Unplaceable → `?<uuid>`.
- Orders:
  - hand and graveyard: `CardsImpl` is a `LinkedHashSet`, so insertion order (draw order)
  - library: `getCardList()`, top first
  - battlefield: `Battlefield.field` is a `LinkedHashMap`, so **order of entering the battlefield** (a permanent
    that leaves and returns goes to the end)
  - stack: `SpellStack` is an `ArrayDeque` and `push` = `addFirst`, so **`S0` is the top**: the spell being cast,
    during its own targets and mana
- Inside an object: the object's id, then `<ref>s` = a stack object's stack ability, then `<ref>a<j>` = its
  abilities (`card.getAbilities(game)` for cards and permanents), each followed by `m<k>` modes. Examples: `B7a3`,
  `H0.7a2`, `S0sm2`.
- `manaPlayer` is a player ref. `manaType` is the enum name.
- Replay `next()` fails on the wrong seat, on a ref that resolves to nothing, or when the log runs out. The player then
  quits (HP:445-448).

## Surprises worth knowing
- DECLARE_ATTACKERS gives both players a priority question every combat, even with no attack. DECLARE_BLOCKERS and
  the damage steps do too whenever there are attackers. Five steps auto-pass. In the recorded games, about 60% of
  all answers are priority passes.
- Neither the F-keys nor any other player action is recorded. Their `int 0` is, so a log made with F-keys won't
  replay.
- Any answer UUID that isn't a game object ends up as `?<uuid>` and breaks the replay. Seen: `RandomDriver` answered
  priority with an MDFC half's id, `?f392…`, and the log recorded it that way. The Done/Cancel ids of CHOOSE_MODE
  can't be named, so "choose one or more" modes can't be replayed.
- Floating mana from earlier actions is never auto-spent (restricted auto-pay plus `setStock`). It needs a
  `manaType` answer.
- A 0-cost spell, or a non-mana ability without mana in its cost, shows the ability picker even when it has one
  option.
- "Select blockers" is asked whenever you have any untapped creature, even if none can legally block.
- `prepareForResponse` moves `priorityPlayerId` to whoever is asked, and the fingerprint shows it.
- `RandomDriver` answers MULTI_AMOUNT with a boolean, which HumanPlayer ignores, so a recorded game with a double
  block or trample overflow would loop until the timeout.
