---
name: Playmat
description: The XMage web client as the player's own playmat. Printed neoprene, ivory ink, one amber for the decision in front of you.
colors:
  decision: "#f0b13d"
  decision-strong: "#ffc65a"
  decision-ink: "#1d1406"
  decision-glow: "rgb(240 177 61 / 0.35)"
  oxblood: "#8a2f2a"
  oxblood-ink: "#f0b3a9"
  danger-text: "#e5877c"
  sage: "#8fc19a"
  brass: "#d6b36a"
  brass-ink: "#e8cf8f"
  copper: "#e0743f"
  copper-ink: "#f1a37a"
  ink: "#e9e4d6"
  ink-strong: "#f6f2e8"
  ink-soft: "rgb(233 228 214 / 0.78)"
  ink-dim: "rgb(233 228 214 / 0.62)"
  ink-faint: "rgb(233 228 214 / 0.3)"
  ink-ghost: "rgb(233 228 214 / 0.12)"
  thread: "#d4ccb6"
  mat-950: "#070c0a"
  mat-900: "#0b120f"
  mat-800: "#0f1714"
  mat-700: "#142019"
  mat-600: "#1c2d27"
  mat-400: "#3a5a51"
  mana-w: "#f3ecd0"
  mana-u: "#6aa3e3"
  mana-b: "#a998b6"
  mana-r: "#e86c4c"
  mana-g: "#6fbb7c"
  mana-c: "#c4bfb3"
typography:
  display:
    fontFamily: "Barlow Condensed, Barlow, ui-sans-serif, system-ui, sans-serif"
    fontSize: "4.5rem"
    fontWeight: 700
    lineHeight: 1.05
    letterSpacing: "0.01em"
  headline:
    fontFamily: "Barlow Condensed, Barlow, ui-sans-serif, system-ui, sans-serif"
    fontSize: "3rem"
    fontWeight: 700
    lineHeight: 1.05
    letterSpacing: "0.01em"
  title:
    fontFamily: "Barlow Condensed, Barlow, ui-sans-serif, system-ui, sans-serif"
    fontSize: "2rem"
    fontWeight: 700
    lineHeight: 1.05
    letterSpacing: "0.08em"
  plate:
    fontFamily: "Barlow Condensed, Barlow, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "0.01em"
  body:
    fontFamily: "Barlow, ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: "tnum"
  body-sm:
    fontFamily: "Barlow, ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.45
  label-print:
    fontFamily: "Barlow Condensed, Barlow, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 700
    letterSpacing: "0.16em"
  label-button:
    fontFamily: "Barlow Condensed, Barlow, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 700
    letterSpacing: "0.08em"
rounded:
  zone: "14px"
  deck-box: "10px"
  control: "8px"
  chip: "999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "24px"
  "6": "32px"
  "7": "48px"
  "8": "64px"
  stitch-inset: "10px"
components:
  button-decision:
    backgroundColor: "{colors.decision}"
    textColor: "{colors.decision-ink}"
    typography: "{typography.label-button}"
    rounded: "{rounded.control}"
    padding: "0 18px"
    height: "40px"
  button-decision-hover:
    backgroundColor: "{colors.decision-strong}"
    textColor: "{colors.decision-ink}"
  button-print:
    backgroundColor: "rgb(233 228 214 / 0.04)"
    textColor: "{colors.ink}"
    typography: "{typography.label-button}"
    rounded: "{rounded.control}"
    padding: "0 18px"
    height: "40px"
  button-print-hover:
    backgroundColor: "rgb(233 228 214 / 0.09)"
    textColor: "{colors.ink}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.ink-dim}"
    typography: "{typography.label-button}"
    rounded: "{rounded.control}"
    padding: "0 10px"
    height: "40px"
  button-danger:
    backgroundColor: "rgb(138 47 42 / 0.2)"
    textColor: "{colors.oxblood-ink}"
    typography: "{typography.label-button}"
    rounded: "{rounded.control}"
    padding: "0 18px"
    height: "40px"
  button-disabled:
    backgroundColor: "transparent"
    textColor: "{colors.ink-dim}"
    rounded: "{rounded.control}"
  field-input:
    backgroundColor: "rgb(7 12 10 / 0.6)"
    textColor: "{colors.ink-strong}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "44px"
  zone-line:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.zone}"
    padding: "{spacing.4}"
  zone-filled:
    backgroundColor: "rgb(7 12 10 / 0.55)"
    textColor: "{colors.ink}"
    rounded: "{rounded.zone}"
    padding: "{spacing.4}"
  dialog:
    backgroundColor: "{colors.mat-700}"
    textColor: "{colors.ink}"
    rounded: "{rounded.zone}"
    width: "min(92vw, 560px)"
  deck-box:
    textColor: "{colors.ink}"
    typography: "{typography.plate}"
    rounded: "{rounded.deck-box}"
    padding: "6px"
  toast:
    backgroundColor: "{colors.mat-600}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    padding: "12px 16px"
  import-sheet:
    backgroundColor: "{colors.mat-700}"
    textColor: "{colors.ink}"
    rounded: "{rounded.zone}"
    width: "min(94vw, 980px)"
    height: "min(88vh, 720px)"
  site-mark:
    backgroundColor: "rgb(233 228 214 / 0.1)"
    textColor: "{colors.ink-strong}"
    typography: "{typography.label-print}"
    rounded: "6px"
    size: "22px / 28px / 56px"
  fix-row:
    backgroundColor: "color-mix(in srgb, {colors.decision} 8%, transparent)"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    padding: "8px 10px"
---

# Design System: Playmat

## Overview

**Creative North Star: "The Player's Own Mat"**

Every screen is the player's playmat: dyed bottle-green neoprene with a fine printed grain, zones and labels screen-printed in slightly translucent ivory ink, a dashed thread stitched 10px in from the viewport edge, and the selected deck's art printed into the mat as a green duotone. Things that sit on the mat (cards, deck boxes, dialogs) are physical objects that cast real shadows. Things that are part of the mat (zones, labels, tabs, outlines) are ink and cast none.

Color carries one meaning. Amber means "you have to decide this now." Everything else is ivory ink on green, with oxblood for damage and destructive actions and sage for good outcomes. State is marked with ink, not just colored: a heavier outline, a printed tick, an underline.

The world explicitly refuses dark-fantasy navy with gold trim over blurred art, and floating glass panels.

**Key Characteristics:**
- Green neoprene ground with authored SVG grain, never flat black and never blur.
- Ivory screen-print ink for all structure, labels and selection.
- One amber, only for the decision in front of the player.
- Barlow Condensed uppercase for anything printed on the mat; Barlow for reading.
- Placed objects cast shadows; printed ink is flat.

## Colors

A monochrome green-and-ivory world with a single hot accent and two quiet semantic hues.

### Primary
- **Decision Amber** (`decision`, with `decision-strong` for the top of the button gradient and `decision-ink` for text on it): the main decision button, selected targets and attackers, chosen hand cards, the draft pick, the live targeting arrow, the player plate and prompt pill when you must act, a deck that's due in an event, and the X-value slider. Nothing else.

### Secondary
- **Oxblood** (`oxblood`, `oxblood-ink`, `danger-text`): damage, attack declarations, destructive buttons, field errors, the error toast.
- **Sage** (`sage`): positive outcomes only (legal deck, victory scores).
- **Brass and Copper** (`brass`, `brass-ink`, `copper`, `copper-ink`): Career's reward metals, nowhere else. Brass is what you earned (coins, XP, rares, achievements, a won verdict); copper is the rarest (mythics, level rewards). Never on a button, never for focus or selection.

### Neutral
- **Neoprene** (`mat-950` to `mat-600`): the ground. `mat-800` is the app body, `mat-700` the dialog stock, `mat-600` toasts and card-back weave, `mat-900`/`mat-950` the match frame vignette and dark wells. `mat-400` is text selection.
- **Ivory Ink** (`ink-strong` for selected and focused, `ink` for body and active, `ink-soft` for secondary copy, `ink-dim` for labels and idle controls, `ink-faint` for zone outlines, `ink-ghost` for hairlines and disabled outlines).
- **Thread** (`thread`): the stitched edge only, at 55% opacity.
- **Mana pips** (`mana-*`): mana symbols only, never UI chrome.

### Named Rules
**The Decision Rule.** Amber means "your decision now." If the player is not being asked to choose this thing at this moment, it is not amber. Focus, selection, filters, tabs, the active nav item and the current turn are ivory ink.

**The One Amber Button Rule.** A screen has at most one amber button visible at a time: its main decision. Dialog footers count as their own screen while open. Every other action is print, quiet or danger.

**The Marked State Rule.** Selected is drawn in ink: a 2 to 2.5px `ink-strong` outline, plus a printed tick (ivory disc, dark check) on items picked from a list or roster. Never signal state with hue alone.

## Typography

**Display Font:** Barlow Condensed (500/600/700), falling back to Barlow
**Body Font:** Barlow (400 to 700), with ui-sans-serif and system fallbacks

**Character:** Condensed caps read as ink pressed through a screen onto the mat; the regular Barlow is for anything the player has to read. Body uses tabular numerals throughout.

### Hierarchy
- **Display** (700, 4.5rem, 1.05): one page title per top-level screen (Events, event page).
- **Headline** (700, 3rem): titles of focused task screens (draft, deck construction).
- **Title** (700, 2rem, uppercase, 0.08em): dialog titles.
- **Plate** (700, 1.5rem, uppercase, line-height 1): deck names on deck boxes and similar name plates; clamps at two lines.
- **Body** (400, 0.9375rem, 1.45): all reading text. `body-sm` (0.8125rem) for hints, notes and meta lines.
- **Label Print** (700, 0.9375rem, uppercase, 0.16em, `ink-dim`): zone names printed on the outline, field labels (600 at 0.8125rem), nav items (at 1.125rem).
- **Label Button** (700, 1.125rem, uppercase, 0.08em): button text; `xl` buttons go to 2rem at 0.16em.

### Named Rules
**The Stage Pixel Rule.** The match is laid out once at 1920x1080 and scaled to the window. HUD type there is set in stage pixels, sized so it never renders below about 11px at 0.53 scale: 21 stage px is the floor, 22 to 24px is the working size for labels and counts. Menus portaled out of the stage use normal rem sizes.

## Layout

App screens sit inside the stitched edge (10px inset) under a thin printed rail: product mark left, Play / Decks / Events / Tables as printed nav labels, settings and profile right. Spacing is a 4px grid (`spacing.1` to `spacing.8`); zones pad 16px, dialogs 24px, gaps between zones 12 to 24px.

The Play screen is the reference composition: deck roster column left, selected deck art printed full-bleed in the right two thirds, the PLAY zone over it with the amber button at its lower right as the largest control.

Desktop first from 1280x720. Two breakpoints exist: 1200px (Home, Tables, Deck builder slim down: narrower roster and deck list, smaller binder cards) and 860px (Login). The match never reflows: it scales the 1920x1080 stage with `min(width/1920, height/1080)` and letterboxes into the mat vignette.

Layers: mat 0, board 10, rail and stitch 40, overlays 100, dialogs 200, toasts 300.

## Elevation & Depth

Hybrid, and the split is semantic. Ink printed on the mat is flat: zones, outlines, labels, tabs, print buttons. Objects placed on the mat cast real, dark, two-part shadows that grow as the object leaves the surface.

### Shadow Vocabulary
- **Placed** (`0 1px 1px rgb(0 0 0 / 0.35), 0 4px 10px rgb(0 0 0 / 0.35)`): cards, deck boxes, the amber button at rest.
- **Lifted** (`0 2px 3px rgb(0 0 0 / 0.35), 0 14px 32px rgb(0 0 0 / 0.5)`): the selected deck box, hovered amber button, toasts, chosen targets.
- **Held** (`0 6px 8px rgb(0 0 0 / 0.35), 0 28px 60px rgb(0 0 0 / 0.55)`): dialogs and menus, things in the hand of the UI.

Decision glow (`0 0 0 3px decision` ring plus a soft `decision-glow` bloom) is the only colored shadow and follows the Decision Rule.

### Named Rules
**The Printed Zone Rule.** Zones are a 1.5px `ink-faint` inset outline with 14px corners, transparent so the mat shows through. Where a zone needs a well (fields, filled zones), it is a dark pressed-in tint with an inner shadow. Never a translucent glass panel, never backdrop blur.

**The Placed Motion Rule.** Things are placed and settle (`cubic-bezier(0.16, 1, 0.3, 1)`, 260ms); hovers lift quickly (140ms). Nothing floats or bobs. Reduced motion collapses everything to 1ms; the mat re-print becomes a crossfade.

## Shapes

Mat-zone corners (14px) for zones and dialogs, softer control corners (8px) for buttons, fields, toasts and menus, 10px for deck boxes, full pills for chips, prompt pills and counters. Cards use real 63:88 proportions with a 4.6% corner and a 2.2% matte sleeve rim in the deck's sleeve color. Outlines on zones, buttons, fields and selected items are inset box-shadows (1.5px for print, 2 to 2.5px for selected), so state changes never shift layout. Tab underlines and hairline dividers are the only plain borders.

## Components

### Buttons
- **Shape:** control corners (8px); heights 30 / 40 / 52 / 72px for sm / md / lg / xl.
- **Decision:** amber gradient from `decision-strong` to `decision`, dark `decision-ink` text, a bevel (light inset top, dark inset bottom) and the placed shadow, like a token on the mat. Hover lifts 1px and adds a 6px amber halo; active presses down 1px.
- **Print (default):** a 1.5px `ink-faint` outline over a 4% ivory tint; hover strengthens the outline to `ink-soft`.
- **Quiet:** text only in `ink-dim`, for cancel and secondary actions.
- **Danger:** oxblood tint with a `danger-text` outline.
- **Disabled:** an empty ink outline (1.5px `ink-ghost`, `ink-dim` text, no fill) for every variant. Never a faded amber. A busy button keeps its color and shows a spinner.
- **Icon button:** 36px square, `ink-dim`, pressed state is a 12% ivory fill with `ink-strong`.

### Chips and Filters
- **Style:** pill or control-cornered tiles, `ink-faint` outline, `ink-dim` text.
- **State:** on is a 2.5px `ink-strong` outline and `ink-strong` text (mana-cost filters, color pips, event kinds, play modes). Tabs and nav use a 2 to 3px ivory underline. Never amber.

### Zones
- **Corner Style:** 14px.
- **Background:** transparent (line) or `rgb(7 12 10 / 0.55)` pressed well (filled).
- **Border:** 1.5px inset ink outline (`ink-faint` for line, `ink-ghost` for filled).
- **Label:** printed in Label Print on the top left, counts or actions on the right of the same row.
- **Internal Padding:** 16px.

### Inputs / Fields
- **Style:** 44px dark pressed well, 8px corners, `ink-ghost` outline with an inner shadow, `ink-strong` text at 1.125rem; label above in field-label caps.
- **Focus:** the outline goes to 2px `ink-strong`. Never amber.
- **Error:** `danger-text` outline and a `danger-text` message under the field.

### Navigation
Printed rail labels in Barlow Condensed caps (1.125rem, wide tracking), `ink-dim` idle, `ink-strong` with a 3px ivory underline that draws in from the left when active.

### Dialogs
Mat-700 stock with grain, 14px corners, `ink-faint` outline and the held shadow over a 72% dark scrim; placed in with a 10px rise. Footer right-aligned: quiet Cancel, then the one decision button. Widths 420 / 560 / 860px.

### Deck Box
A sleeve-colored shell with a light sheen, a 74px art window and a printed name plate (Plate type plus a one-line `body-sm` note with mana pips). Hover slides 3px right; selected slides 10px, gains a 2.5px `ink-strong` ring, the lifted shadow and a printed tick at top right.

### Card Face
Real card proportions in a matte sleeve rim, placed shadow. Until the image loads, a text frame (name, type line, P/T) in container-query sizes. Card backs are the project's own printed weave on mat green, not WotC art.

### Import Sheet
The one place decks come in: a wide dialog (980 × 720px, same stock, outline and scrim as Dialogs) with steps instead of pages. **Input**: one field takes a link, a list or a dropped file; a recognized site shows as a placed chip in the field's corner ("Archidekt deck · reads in one step"), and the row of every site's mark below lights the matching one. **Loading**: a ghost deck box whose plate shimmers, never a spinner. **Copy and paste** (sites that block other apps): the site's mark large, three numbered steps, and a hatched paste target that turns amber with a soft pulse when the player comes back to the tab, because finishing is now their decision. **Preview**: left, the deck box exactly as it will sit on the shelf over a Mat Print of its cover that fades out before the counts, then counts in Plate numerals, commanders as small printed plates, the curve and any legality problems said in second person in `danger-text` on plain mat; right, the name as a Display-size input, the source line, format and legality chip, printings choice, cards to fix, and the card list grouped like the builder's. Esc steps back before it closes; Enter saves. The footer carries at most one amber button: Save and play when the deck is ready, Save when it can't be played yet (Save and play goes to an empty outline), none while cards need a look. It renders through `ui/Dialog` at `width="xl"` with `onBack`.

### Site Mark
A deck site is never shown by its logo. Its mark is two letters screen-printed in ivory ink inside a 6px rounded square with an `ink-faint` outline, in three sizes (22, 28, 56px). In the input's site row, marks and names are printed straight on the mat (no pills, since they aren't buttons); a sage link icon means "reads in one step", explained by a visible legend line.

### Fix Row
A card that didn't match the card database: an amber-tinted plate with an amber outline, count in `decision-strong`, the name, then a small amber "Find it" and a quiet "Leave out". Opening it lays out printings as small mat-600 tiles, closest name first. Amber here is correct: each row is a decision only the player can make.

### Drop Target
While a deck file, link or list is dragged over a shelf, the zone's printed outline thickens to 2px `ink-strong` and a dark plate says what will happen ("Drop a deck file, link or list"). A deck that has just arrived slides down onto the shelf over 620ms.

### Mat Print (signature)
The selected deck's art printed into the mat in three separations (shadow, light, key) under a mat-green dye and a 4px halftone, masked to fade into the plain mat. Selecting a deck re-prints it: separations slide into register over 620ms.

## Career: game mode

Career is the same playmat with the house lights down: a single-player game mode, not a set of web pages. Everything above still holds; this section adds what a game screen needs.

### Shell and HUD
Every `/career` route renders inside `CareerShell`: full bleed, the app's header gone. The ground is a Mat Print of the card in front of the player (the next opponent, the chapter's boss, the result's opponent), vignetted toward `void` at the top and bottom. A HUD strip runs across the top: back (Escape does the same), "Career" and the screen's name in Plate caps, the wallet (coins counting up when they change, rare and mythic wildcards, packs to open), the music toggle, settings, and the Level Medallion: the avatar in a brass XP ring with the level on a badge. The hub's backdrop turns through the next duel and the modes' art every nine seconds; still, it stays on the first.

### Portrait Card
An opponent is their signature card: the art crop in a frame (cover from Career content, or the highest-rarity, highest-cost card of their deck), a name plate in Plate caps with colour pips, and a foot for wins and pay. Sizes sm / md / lg through `--portrait-w`. Locked frames are sealed (dimmed, lock seal); a boss gets a crown; the next one is selected in ink. Until the art loads, the drawn portrait stands in.

### Scenes
- **Hub**: the next duel as a large portrait with one amber Play, nine mode tiles of placed art (state on each: next node, run record, packs waiting), quests and the weekly goal at the side.
- **Ladder**: tiers stacked from the champions down, each a numbered rung and a row of portraits; a sealed tier shows the wins still needed as pips.
- **Versus**: you left, the opponent right, sliding in; VS stamps between them; stakes as brass chips; their opening line; one amber Fight that stays busy while the table is set.
- **Results**: the verdict stamped over the opponent's frame (brass for victory, ink for defeat) and their last word, then coins count up, the XP bar sweeps level by level, quests tick in, prizes deal in as brass-edged cards. Every beat skips on Skip, Space or Enter.
- **Pack**: full screen. The sealed wrapper (its hue from the set code, never the card art) tears in two, the cards deal in face down and turn one by one; rares glow brass, mythics copper with a jolt of the table. Reveal all and the summary close it.
- **Binder**: a shelf of set spines with completion, and the open set two pages at a time, nine pockets each. Owned cards sit in pockets with playset pips; the rest are dashed outlines with number and name. Cards new since the last look catch a brass shine once and keep a NEW tag for the visit.
- **Campaign**: the chapter's path with the boss at its end, the boss's art behind it; each duel's opponent speaks in a speech card (before the duel, then the parting word once it's won).
- **Ceremonies**: a new tier, chapter, shop set or achievement gets one full-screen moment the first time (art or crest, kicker, title, one line, Continue). They queue and never repeat.
- **Capsule machine**: a painted object placed on the mat (oxblood enamel, brass, a glass globe; art/cabinet.mjs), not a drawing. Its capsules are live in the globe: they settle, jump when the glass is tapped, and tumble while the crank wheel turns, one click a notch. One falls through, bounces down the chute and comes up to the middle of the screen, where the player prises it open. Escalation is by what it holds: an ordinary capsule rocks now and then and opens with a thread of light; one that finishes a set leaks its colour at the seam, can't keep still, holds the light a beat and shakes the screen; one that finishes the machine does all of it in gold. Reduced motion lays the heap still and lands every beat at once.

### Motion
One kit (`career/motion.ts`, `motion.module.css`): deal (staggered rise, capped at ten steps), scene (a screen rises 12px and settles), stamp (lands large and settles), pulse (an ivory breath, three times), count-up, and sequences of beats. No move blocks input for more than 1.2s, everything long can be skipped, and reduced motion or the animations setting turns every move into a plain appearance.

### Sound and music
Career cues are synthesized like the match's: tap, whoosh, coin and coins, XP, level-up, unlock, tear, flip, rare, mythic, versus, win, loss. A generative pad (Am, F, C, G with pentatonic bells) plays while Career is open and no game is, under its own toggle and volume in Settings, never louder than the effects.

### Voices
Each Career opponent has five lines in Career content (intro, big play, low life, win, lose). In a Career game they appear as the opponent's speech bubble, each at most once; muting the opponent hides them; free play never has them.

### Navigation
Arrow keys move a visible ink focus between anything marked `data-nav`, Enter takes it, Escape goes back. A gamepad does the same: d-pad or stick to move, A to take, B to go back.

### Named Rules
**The Reward Metal Rule.** Brass and copper are for what the player got, never for what they must decide. The decision on a Career screen is still the one amber button.

**The Skippable Rule.** Every Career animation can be skipped or is shorter than a beat; nothing makes the player wait to play.

## Do's and Don'ts

### Do:
- **Do** build new screens from Zone, Button, Field, Dialog and DeckBox before writing new surfaces.
- **Do** give each screen exactly one amber button, and only when there is a decision to make.
- **Do** mark selection with an `ink-strong` outline and, for list or roster picks, the printed tick.
- **Do** keep the stitched edge and mat grain on every top-level screen.
- **Do** set match HUD type in stage pixels at 21px or larger.
- **Do** use placed / lifted / held shadows only for objects on the mat, and keep printed ink flat.
- **Do** show Career opponents by their card's art in a Portrait Card, and let every Career sequence be skipped.

### Don't:
- **Don't** use amber for focus, hover, tabs, filters, the active nav item, decoration or celebration.
- **Don't** fade a disabled button; draw it as an empty ink outline.
- **Don't** make glass panels, backdrop blur, or navy-and-gold fantasy chrome.
- **Don't** outline zones, buttons or selected items with layout-shifting borders; use the inset outline weights above.
- **Don't** use mana colors for UI state.
- **Don't** use brass or copper outside Career's rewards, or put either on a button.
