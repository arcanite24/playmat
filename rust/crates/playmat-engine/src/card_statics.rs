//! Static handlers (docs/rust/Engine.md, "Per-card handlers"): a card's own continuous, replacement, cost,
//! restriction, rule-modifying and as-though effect classes, written in Rust like the one-shot ones. A static
//! ability's effect with a handler is `{"class": "CardStatic", "handler": …}` in cards.json (`EffectDef::CardStatic`);
//! the engine asks the handlers where Java's `ContinuousEffects` would:
//!
//! - **layers**: `Handler::continuous(name, affects, Layer::…)`. The layer (and sublayer) is the `Layer` variant:
//!   4 `Type` (subtypes added), 6 `Ability`, 7b `SetPt`, 7c `ModifyPt`, read by `Game::has_subtype`,
//!   `granted_abilities`/`has_ability` and `power_toughness`, after the engine's own static effects of that layer.
//!   Layer 7b takes the effect's timestamp (its source's, `Game::static_at`, or a resolved one's) in timestamp order
//!   with the others; layer 6 abilities of statics older than a "loses all abilities" are lost with the rest.
//! - **replacements**: `Handler::replacement(name, &[EventType::…], applies, replace)`, at the engine's event points
//!   (`replace_event`): each one applies once per event (`Event::applied`), in battlefield order, and two or more that
//!   apply owe Java's "which first" question to the event's player, as the engine's own replacements do.
//! - **costs**: `Handler::cost(name, CostKind::…, applies, apply)`, in `CostModificationType` order (increases, then
//!   reductions, then sets) after the engine's own of that kind, for spells (`rules::modified_cost_of`, the spell's
//!   own static abilities too: "this spell costs {2} less") and activated abilities (`adjusted_mana_cost_own`).
//! - **restrictions**: `Handler::restriction(name, applies, &[Restrict::…])`: `rules::can_block`, `can_attack`.
//! - **rule-modifying**: `Handler::rule(name, Rule::CantCast(f))`: `rules::castable` (and the cast itself).
//! - **as-though**: `Handler::as_though(name, AsThough::Flash(f))`: `rules::castable`'s timing.
//!
//! A handler's functions get a `StaticCx`: the effect's source and controller (Java's `source`), its timestamp, and
//! the Java the transpiler translated for it. The same `CardStatic` resolving as a one-shot effect (Emergence Zone's
//! "you may cast spells this turn as though they had flash") lasts until the turn's cleanup (`Game::card_statics`).
//! Keep a handler's functions from asking about what their own layer changes (a 7c boost reading power), as Java's
//! layer order does.

use super::*;
use crate::card::{AbilityDef, CardDef, Filter, ManaCost};
use crate::state::{ResolvedStatic, Zone};

/// What a static handler's functions get: Java's `source` (the static ability's object and controller), when the
/// effect started (its timestamp), the Java constructor's arguments as their text, and what the transpiler made of
/// the Java the handler asked for, in order
#[allow(dead_code)]
pub(crate) struct StaticCx<'a> {
    pub source: ObjId,
    pub controller: Seat,
    pub timestamp: u64,
    pub args: &'a [String],
    pub effects: &'a [EffectDef],
    pub filters: &'a [Filter],
    pub abilities: &'a [AbilityDef],
}

#[allow(dead_code)] // a kit for the handlers: not every handler uses every part
impl StaticCx<'_> {
    /// Java argument `i` as a number
    pub fn arg_int(&self, i: usize) -> Option<i32> {
        self.args.get(i)?.trim().parse().ok()
    }

    /// Java argument `i` as a boolean (`false` when it's missing)
    pub fn arg_bool(&self, i: usize) -> bool {
        self.args.get(i).is_some_and(|a| a.trim() == "true")
    }

    /// `filters[i].match(object, source.getControllerId(), source, game)`
    pub fn matches(&self, game: &Game, i: usize, id: ObjId) -> bool {
        self.filters.get(i).is_some_and(|f| game.filter_matches(f, self.controller, Some(self.source), id))
    }

    /// `game.getOpponents(source.getControllerId()).contains(seat)`
    pub fn opponent(&self, seat: Seat) -> bool {
        seat != self.controller
    }
}

/// The objects a continuous effect or restriction applies to
pub(crate) type AffectsFn = fn(&Game, &StaticCx, ObjId) -> bool;

/// A continuous effect's layer (Java's `Layer` and `SubLayer`), with what it does to an object it affects there
#[allow(dead_code)]
pub(crate) enum Layer {
    /// layer 4, `TypeChangingEffects_4`: the subtypes it adds
    Type(fn(&Game, &StaticCx, ObjId) -> Vec<String>),
    /// layer 6, `AbilityAddingRemovingEffects_6`: the abilities it adds
    Ability(fn(&Game, &StaticCx, ObjId) -> Vec<AbilityDef>),
    /// layer 7b, `PTChangingEffects_7` / `SetPT_7b`: the base power and toughness it sets
    SetPt(fn(&Game, &StaticCx, ObjId) -> (i32, i32)),
    /// layer 7c, `PTChangingEffects_7` / `ModifyPT_7c`: what it adds to power and toughness
    ModifyPt(fn(&Game, &StaticCx, ObjId) -> (i32, i32)),
}

impl Layer {
    /// Java's names for it
    #[allow(dead_code)]
    pub fn java(&self) -> (&'static str, &'static str) {
        match self {
            Layer::Type(_) => ("TypeChangingEffects_4", "NA"),
            Layer::Ability(_) => ("AbilityAddingRemovingEffects_6", "NA"),
            Layer::SetPt(_) => ("PTChangingEffects_7", "SetPT_7b"),
            Layer::ModifyPt(_) => ("PTChangingEffects_7", "ModifyPT_7c"),
        }
    }
}

pub(crate) struct Continuous {
    pub affects: AffectsFn,
    pub layer: Layer,
}

/// The events a replacement handler can check (Java's `EventType`), each where the engine has that event
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum EventType {
    /// `DRAW_CARD`: `player` would draw a card (`rules::draw_one`)
    DrawCard,
    /// `GAIN_LIFE`: `player` would gain `amount` life (`rules::gain_life`)
    GainLife,
    /// `SURVEIL`: `player` would surveil `amount` (`SurveilEffect`)
    Surveil,
    /// `NUMBER_OF_TRIGGERS`: `source`'s triggered ability, controlled by `player`, triggers (`Game::trigger_cause`
    /// says what caused it). The engine counts each one that applies as one more time (`amount + 1`, as every
    /// Java one does): their `replace` isn't called (`Game::additional_triggers` can't change the game)
    NumberOfTriggers,
}

/// The event a replacement handler sees and may change (Java's `GameEvent`)
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct Event {
    pub kind: EventType,
    /// `getPlayerId()`
    pub player: Seat,
    /// `getTargetId()` and `getSourceId()`, where the event has them
    pub target: Option<ObjId>,
    pub source: Option<ObjId>,
    /// `getAmount()`
    pub amount: i32,
    /// `getAppliedEffects()`: the sources of the replacement effects already applied to it
    pub applied: Vec<ObjId>,
}

impl Event {
    pub fn new(kind: EventType, player: Seat) -> Event {
        Event { kind, player, target: None, source: None, amount: 0, applied: Vec::new() }
    }
}

/// `applies(event, source, game)`
pub(crate) type ReplacesFn = fn(&Game, &StaticCx, &Event) -> bool;
/// `replaceEvent(event, source, game)`: true when the event is replaced (it doesn't happen), false when it goes on
/// as changed
pub(crate) type ReplaceFn = fn(&mut Game, &StaticCx, &mut Event) -> bool;

pub(crate) struct Replacement {
    pub events: &'static [EventType],
    pub applies: ReplacesFn,
    pub replace: ReplaceFn,
}

/// `CostModificationType`: the order cost changes apply in
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum CostKind {
    Increase,
    Reduce,
    Set,
}

/// The spell or ability whose cost a cost handler changes (Java's `abilityToModify`)
#[allow(dead_code)]
pub(crate) struct CostCx<'a> {
    /// who casts or activates it
    pub seat: Seat,
    /// the card cast, or the ability's object
    pub card: ObjId,
    /// for a spell, the face cast (an adventure's, a split half's)
    pub face: &'a CardDef,
    /// None for a spell, else the activated ability
    pub ability: Option<&'a AbilityDef>,
}

pub(crate) type CostAppliesFn = fn(&Game, &StaticCx, &CostCx) -> bool;
pub(crate) type CostApplyFn = fn(&Game, &StaticCx, &CostCx, &mut ManaCost);

pub(crate) struct CostMod {
    pub kind: CostKind,
    pub applies: CostAppliesFn,
    pub apply: CostApplyFn,
}

/// What a restriction stops (Java's `RestrictionEffect` methods), for a permanent it applies to
#[allow(dead_code, clippy::enum_variant_names)]
pub(crate) enum Restrict {
    /// `canBeBlocked(attacker, blocker)` is false: (attacker, blocker)
    CantBeBlocked(fn(&Game, &StaticCx, ObjId, ObjId) -> bool),
    /// `canBlock(attacker, blocker)` is false: (blocker, attacker)
    CantBlock(fn(&Game, &StaticCx, ObjId, ObjId) -> bool),
    /// `canAttack` is false
    CantAttack(fn(&Game, &StaticCx, ObjId) -> bool),
}

pub(crate) struct Restriction {
    /// `applies(permanent)`: the attacker for `CantBeBlocked` and `CantAttack`, the blocker for `CantBlock`
    pub applies: AffectsFn,
    pub restrict: &'static [Restrict],
}

/// What a rule-modifying effect stops
#[allow(dead_code)]
pub(crate) enum Rule {
    /// `CAST_SPELL` (and `CAST_SPELL_LATE`): `seat` can't cast the card `id` as `face`
    CantCast(fn(&Game, &StaticCx, Seat, ObjId, &CardDef) -> bool),
}

/// What an as-though effect allows
#[allow(dead_code)]
pub(crate) enum AsThough {
    /// `CAST_AS_THOUGH_IT_HAD_FLASH`: `seat` may cast the card `id` as `face` as though it had flash
    Flash(fn(&Game, &StaticCx, Seat, ObjId, &CardDef) -> bool),
}

/// A cards.json reference to a static handler (`EffectDef::CardStatic`), checked as the card loads
#[derive(Clone, Copy)]
pub struct StaticHandler(&'static Handler);

impl StaticHandler {
    pub fn java(&self) -> &'static str {
        self.0.java
    }

    pub(crate) fn hook(&self) -> &'static Hook {
        &self.0.hook
    }
}

impl std::fmt::Debug for StaticHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.java)
    }
}

impl PartialEq for StaticHandler {
    fn eq(&self, other: &Self) -> bool {
        self.0.java == other.0.java
    }
}

impl<'de> serde::Deserialize<'de> for StaticHandler {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        match find(&name) {
            Some(
                h @ Handler {
                    hook:
                        Hook::Continuous(_)
                        | Hook::Replacement(_)
                        | Hook::Cost(_)
                        | Hook::Restriction(_)
                        | Hook::Rule(_)
                        | Hook::AsThough(_),
                    ..
                },
            ) => Ok(StaticHandler(h)),
            Some(_) => Err(serde::de::Error::custom(format!("card handler {name} isn't a static one"))),
            None => Err(serde::de::Error::custom(format!("no card handler {name}"))),
        }
    }
}

// ------------------------------------------------------------------------------------------------- the statics in play

fn has_card_static(e: &EffectDef) -> bool {
    match e {
        EffectDef::CardStatic { .. } => true,
        EffectDef::ConditionalContinuousEffect { effect, .. } => has_card_static(effect),
        _ => false,
    }
}

/// One `CardStatic` and the static ability it's in: `f(hook, cx, effect)`
fn visit(
    game: &Game,
    e: &EffectDef,
    source: ObjId,
    controller: Seat,
    timestamp: u64,
    f: &mut dyn FnMut(&'static Hook, &StaticCx, &EffectDef),
) {
    match e {
        EffectDef::CardStatic { handler, args, effects, filters, abilities } => {
            let cx = StaticCx { source, controller, timestamp, args, effects, filters, abilities };
            f(handler.hook(), &cx, e);
        }
        // the condition only for a handler's effect: a condition can ask the queries the handlers answer
        EffectDef::ConditionalContinuousEffect { effect, condition }
            if has_card_static(effect) && game.condition_holds(Some(condition), controller, source) =>
        {
            visit(game, effect, source, controller, timestamp, f);
        }
        _ => {}
    }
}

/// Every static handler's effect in play, in timestamp order: the static abilities of permanents (and emblems),
/// those a resolved effect gave a permanent, then those that resolved for a while (`Game::card_statics`)
pub(crate) fn each_static(game: &Game, f: &mut dyn FnMut(&'static Hook, &StaticCx, &EffectDef)) {
    for src in game.battlefield.iter().chain(game.command.iter()) {
        let o = game.obj(*src);
        for a in &o.def.abilities {
            let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
            for e in effects {
                visit(game, e, *src, o.controller, game.static_at(*src), f);
            }
        }
    }
    for g in game.grants.iter() {
        let AbilityDef::SimpleStaticAbility { effects, .. } = &g.ability else { continue };
        let o = game.obj(g.target);
        if o.zone == Zone::Battlefield && o.zcc == g.zcc {
            for e in effects {
                visit(game, e, g.target, o.controller, game.static_at(g.target), f);
            }
        }
    }
    for r in game.card_statics.iter() {
        visit(game, &r.effect, r.source, r.controller, r.timestamp, f);
    }
}

/// A card's own static abilities' handlers that work in every zone (`Handler::any_zone`: "this spell costs {2} less
/// to cast"), while it isn't on the battlefield
fn each_own_static(game: &Game, id: ObjId, f: &mut dyn FnMut(&'static Hook, &StaticCx, &EffectDef)) {
    let o = game.obj(id);
    if o.zone == Zone::Battlefield {
        return;
    }
    for a in &o.def.abilities {
        let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
        for e in effects {
            if let EffectDef::CardStatic { handler, .. } = e {
                if handler.0.any_zone {
                    visit(game, e, id, o.controller, 0, f);
                }
            }
        }
    }
}

/// A `CardStatic` resolving as a one-shot effect: it lasts until the turn's cleanup
pub(crate) fn resolve_static(game: &mut Game, effect: &EffectDef, source: ObjId, controller: Seat) {
    let timestamp = game.timestamp();
    game.card_statics.push(ResolvedStatic { effect: effect.clone(), source, controller, timestamp });
}

// ------------------------------------------------------------------------------------------------- what they change

/// Layer 7c: what the handlers add to `id`'s power and toughness
pub(crate) fn modify_pt(game: &Game, id: ObjId) -> (i32, i32) {
    let mut out = (0, 0);
    each_static(game, &mut |h, cx, _| {
        if let Hook::Continuous(Continuous { affects, layer: Layer::ModifyPt(f) }) = h {
            if affects(game, cx, id) {
                let (p, t) = f(game, cx, id);
                out.0 += p;
                out.1 += t;
            }
        }
    });
    out
}

/// Layer 7b: the base power and toughness the handlers set on `id`, each with its timestamp
pub(crate) fn set_pt(game: &Game, id: ObjId) -> Vec<(u64, i32, i32)> {
    let mut out = Vec::new();
    each_static(game, &mut |h, cx, _| {
        if let Hook::Continuous(Continuous { affects, layer: Layer::SetPt(f) }) = h {
            if affects(game, cx, id) {
                let (p, t) = f(game, cx, id);
                out.push((cx.timestamp, p, t));
            }
        }
    });
    out
}

/// Layer 6: the abilities the handlers give `id` (none from statics older than `lost_at`, when it lost all its
/// abilities then)
pub(crate) fn abilities(game: &Game, id: ObjId, lost_at: Option<u64>) -> Vec<AbilityDef> {
    let mut out = Vec::new();
    each_static(game, &mut |h, cx, _| {
        if let Hook::Continuous(Continuous { affects, layer: Layer::Ability(f) }) = h {
            if !lost_at.is_some_and(|at| cx.timestamp < at) && affects(game, cx, id) {
                out.extend(f(game, cx, id));
            }
        }
    });
    out
}

/// Layer 4: whether a handler gives `id` the subtype `s`. A handler's own test asking about subtypes sees only the
/// others' (no handler's), so a filter of subtypes can't loop.
pub(crate) fn has_subtype(game: &Game, id: ObjId, s: &str) -> bool {
    thread_local!(static INSIDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    if INSIDE.with(|x| x.replace(true)) {
        return false;
    }
    let out = has_subtype_now(game, id, s);
    INSIDE.with(|x| x.set(false));
    out
}

fn has_subtype_now(game: &Game, id: ObjId, s: &str) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Continuous(Continuous { affects, layer: Layer::Type(f) }) = h {
            if !out && affects(game, cx, id) && f(game, cx, id).iter().any(|x| x == s) {
                out = true;
            }
        }
    });
    out
}

/// Whether a handler's restriction says `blocker` can't block `attacker`
pub(crate) fn cant_block(game: &Game, blocker: ObjId, attacker: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if out {
            return;
        }
        if let Hook::Restriction(Restriction { applies, restrict }) = h {
            out = restrict.iter().any(|r| match r {
                Restrict::CantBeBlocked(f) => applies(game, cx, attacker) && f(game, cx, attacker, blocker),
                Restrict::CantBlock(f) => applies(game, cx, blocker) && f(game, cx, blocker, attacker),
                Restrict::CantAttack(_) => false,
            });
        }
    });
    out
}

/// Whether a handler's restriction says `attacker` can't attack
pub(crate) fn cant_attack(game: &Game, attacker: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if out {
            return;
        }
        if let Hook::Restriction(Restriction { applies, restrict }) = h {
            out = restrict
                .iter()
                .any(|r| matches!(r, Restrict::CantAttack(f) if applies(game, cx, attacker) && f(game, cx, attacker)));
        }
    });
    out
}

/// Whether a rule-modifying handler stops `seat` casting `id` as `face` (the card's own too: "you can't cast this
/// spell unless …")
pub(crate) fn cant_cast(game: &Game, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    let mut out = false;
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        if let Hook::Rule(Rule::CantCast(f)) = h {
            out = out || f(game, cx, seat, id, face);
        }
    };
    each_static(game, &mut check);
    each_own_static(game, id, &mut check);
    out
}

/// Whether an as-though handler lets `seat` cast `id` as `face` as though it had flash
pub(crate) fn flash(game: &Game, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    let mut out = false;
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        if let Hook::AsThough(AsThough::Flash(f)) = h {
            out = out || f(game, cx, seat, id, face);
        }
    };
    each_static(game, &mut check);
    each_own_static(game, id, &mut check);
    out
}

/// The cost changes of `kind` the handlers make to a spell or ability (and, for a spell, its own static abilities')
pub(crate) fn change_cost(game: &Game, kind: CostKind, of: &CostCx, cost: &mut ManaCost) {
    let mut change = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        if let Hook::Cost(CostMod { kind: k, applies, apply }) = h {
            if *k == kind && applies(game, cx, of) {
                apply(game, cx, of, cost);
            }
        }
    };
    each_static(game, &mut change);
    if of.ability.is_none() {
        each_own_static(game, of.card, &mut change);
    }
}

/// Whether a replacement handler for events of `kind` is in play
pub(crate) fn any_replacement(game: &Game, kind: EventType) -> bool {
    let mut out = false;
    each_static(game, &mut |h, _, _| {
        out = out || matches!(h, Hook::Replacement(Replacement { events, .. }) if events.contains(&kind));
    });
    out
}

/// `NUMBER_OF_TRIGGERS`: how many more times the replacement handlers make the waiting trigger `t` trigger
pub(crate) fn extra_triggers(game: &Game, t: &StackItem) -> usize {
    let ev = Event { source: Some(t.card), amount: 1, ..Event::new(EventType::NumberOfTriggers, t.controller) };
    let mut n = 0;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Replacement(Replacement { events, applies, .. }) = h {
            if events.contains(&ev.kind) && applies(game, cx, &ev) {
                n += 1;
            }
        }
    });
    n
}

/// The replacement handlers for `ev` (Java's `ContinuousEffects.replaceEvent`), with `others` more of the engine's
/// own applying to it: each applies once, in timestamp order; two or more owe the event's player the question which
/// applies first. True when one replaced the event (it doesn't happen).
pub(crate) fn replace_event(game: &mut Game, ev: &mut Event, others: usize) -> bool {
    loop {
        let mut found: Vec<(EffectDef, ObjId, Seat, u64)> = Vec::new();
        each_static(game, &mut |h, cx, e| {
            if let Hook::Replacement(Replacement { events, applies, .. }) = h {
                if events.contains(&ev.kind) && !ev.applied.contains(&cx.source) && applies(game, cx, ev) {
                    found.push((e.clone(), cx.source, cx.controller, cx.timestamp));
                }
            }
        });
        if found.is_empty() {
            return false;
        }
        if found.len() + others > 1 {
            game.owed_replacement_choices.borrow_mut().push(ev.player);
        }
        let (e, source, controller, timestamp) = found.swap_remove(0);
        let EffectDef::CardStatic { handler, args, effects, filters, abilities } = &e else { unreachable!() };
        let Hook::Replacement(r) = handler.hook() else { unreachable!() };
        let cx = StaticCx { source, controller, timestamp, args, effects, filters, abilities };
        ev.applied.push(source);
        if (r.replace)(game, &cx, ev) {
            return true;
        }
    }
}
