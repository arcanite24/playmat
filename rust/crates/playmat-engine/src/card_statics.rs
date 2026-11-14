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
//!   apply owe Java's "which first" question to the event's player, as the engine's own replacements do. The events:
//!   drawing, gaining life, surveilling, a trigger's count, damage to a player or a permanent (prevention and
//!   redirection: `StaticCx::prevent`), an object changing zones (`Event::to`: "exile it instead") and a permanent
//!   entering (its "enters with" counters).
//! - **mana kept**: `Handler::keeps_mana(name, f)`: the kinds of mana a player doesn't lose as steps and phases end
//!   (`YouDontLoseManaEffect`), read as the pool empties.
//! - **costs**: `Handler::cost(name, CostKind::…, applies, apply)`, in `CostModificationType` order (increases, then
//!   reductions, then sets) after the engine's own of that kind, for spells (`rules::modified_cost_of`, the spell's
//!   own static abilities too: "this spell costs {2} less") and activated abilities (`adjusted_mana_cost_own`).
//! - **restrictions**: `Handler::restriction(name, applies, &[Restrict::…])`: `rules::can_block`, `can_attack`.
//! - **rule-modifying**: `Handler::rule(name, Rule::CantCast(f))`: `rules::castable` (and the cast itself);
//!   `Rule::CantActivate(f)`: `rules::activatable` (activated abilities that aren't mana abilities);
//!   `Rule::CantLoseOrWin(f)`: the state-based loss check and `rules::win_game`.
//! - **as-though**: `Handler::as_though(name, AsThough::Flash(f))`: `rules::castable`'s timing;
//!   `AsThough::Haste(f)`: a {T} cost's summoning sickness (`rules::can_pay_cost`, mana abilities);
//!   `AsThough::Hexproof(f)`: hexproof, as targets are chosen (`rules::possible_targets`, `targetable`).
//!
//! A handler's functions get a `StaticCx`: the effect's source and controller (Java's `source`), its timestamp, and
//! the Java the transpiler translated for it. The same `CardStatic` resolving as a one-shot effect (Emergence Zone's
//! "you may cast spells this turn as though they had flash") lasts until the turn's cleanup (`Game::card_statics`),
//! with the resolving ability's targets as its own (`StaticCx::targets`: Java's target pointer) and, with
//! `Handler::on_start`, what its `init` chose (Deflecting Palm's source).
//! Keep a handler's functions from asking about what their own layer changes (a 7c boost reading power), as Java's
//! layer order does.

use super::*;
use crate::card::{AbilityDef, CardDef, Filter, ManaCost};
use crate::state::{ResolvedStatic, Target, Zone};

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
    /// a resolved effect's targets (Java's target pointer), each with its zone-change count as it resolved (0 for
    /// a player); none in a static ability
    pub targets: &'a [(Target, u32)],
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

    /// the object target `i` points at, while it's still that object (`getFirstTarget` of a fixed target pointer)
    pub fn target_obj(&self, game: &Game, i: usize) -> Option<ObjId> {
        match self.targets.get(i)? {
            (Target::Obj(o), zcc) if game.obj(*o).zcc == *zcc => Some(*o),
            _ => None,
        }
    }

    /// the player target `i` is
    pub fn target_player(&self, i: usize) -> Option<Seat> {
        match self.targets.get(i)? {
            (Target::Player(p), _) => Some(*p),
            _ => None,
        }
    }

    /// what's left of this effect's amount (a consumable prevention shield's `amountToPrevent`), `initial` before
    /// it's used
    pub fn left(&self, game: &Game, initial: i32) -> i32 {
        game.static_amounts
            .iter()
            .find(|(o, t, _)| *o == self.source && *t == self.timestamp)
            .map_or(initial, |(_, _, n)| *n)
    }

    /// sets what's left of this effect's amount
    pub fn set_left(&self, game: &mut Game, n: i32) {
        let key = (self.source, self.timestamp);
        match game.static_amounts.iter_mut().find(|(o, t, _)| (*o, *t) == key) {
            Some(e) => e.2 = n,
            None => game.static_amounts.push((key.0, key.1, n)),
        }
    }

    /// `PreventionEffectImpl.applies`: a damage event whose damage can be prevented, combat damage if `only_combat`
    pub fn preventable(&self, ev: &Event, only_combat: bool) -> bool {
        matches!(ev.kind, EventType::DamagePlayer | EventType::DamagePermanent)
            && ev.preventable
            && (!only_combat || ev.combat)
    }

    /// `game.preventDamage`: prevents up to `n` of the event's damage (all of it: `i32::MAX`), and returns how much
    /// it prevented
    pub fn prevent(&self, ev: &mut Event, n: i32) -> i32 {
        if !ev.preventable {
            return 0;
        }
        let p = ev.amount.min(n).max(0);
        ev.amount -= p;
        p
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
    /// layer 7b, the base toughness alone (`getToughness().setModifiedBaseValue`: Maha, Its Feathers Night)
    SetToughness(fn(&Game, &StaticCx, ObjId) -> i32),
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
            Layer::SetPt(_) | Layer::SetToughness(_) => ("PTChangingEffects_7", "SetPT_7b"),
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
    /// `TAPPED_FOR_MANA`: `target`, controlled by `player`, is tapped for `amount` mana by one of its mana
    /// abilities (`rules::activate_mana_paying`). A `replace` that sets `colorless` makes it produce that much {C}
    /// instead of any other type and amount (Damping Sphere)
    TappedForMana,
    /// `NUMBER_OF_TRIGGERS`: `source`'s triggered ability, controlled by `player`, triggers (`Game::trigger_cause`
    /// says what caused it). The engine counts each one that applies as one more time (`amount + 1`, as every
    /// Java one does): their `replace` isn't called (`Game::additional_triggers` can't change the game)
    NumberOfTriggers,
    /// `DAMAGE_PLAYER`: `amount` damage from `source` would be dealt to `player` (`rules::deal_damage`), `combat` or
    /// not, `preventable` unless damage can't be prevented. Lower `amount` to prevent some (`StaticCx::prevent`), or
    /// deal it elsewhere and return "replaced" to redirect it.
    DamagePlayer,
    /// `DAMAGE_PERMANENT`: the same, to the permanent `target`; `player` is its controller
    DamagePermanent,
    /// `ZONE_CHANGE`: `target` would move from `from` to `to` (`Game::move_to`); `player` is its controller on the
    /// battlefield, else its owner. Change `to` to move it elsewhere instead ("exile it instead").
    ZoneChange,
    /// `ENTERS_THE_BATTLEFIELD`: `target` is entering under `player`'s control (`rules::enter_battlefield`), already
    /// where its "enters with" counters go (`add_counters_by`); the entering permanent's own statics don't apply
    EntersBattlefield,
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
    /// damage: `isCombatDamage()`, and `getFlag()` (it can be prevented)
    pub combat: bool,
    pub preventable: bool,
    /// a zone change's `getFromZone()` and `getToZone()`
    pub from: Option<Zone>,
    pub to: Option<Zone>,
    /// `TappedForMana`: the {C} it produces instead (`ManaEvent.getMana().setToMana`)
    pub colorless: Option<u32>,
}

impl Event {
    pub fn new(kind: EventType, player: Seat) -> Event {
        Event {
            kind,
            player,
            target: None,
            source: None,
            amount: 0,
            applied: Vec::new(),
            combat: false,
            preventable: false,
            from: None,
            to: None,
            colorless: None,
        }
    }
}

/// A Java number argument: `Integer.MAX_VALUE` is `i32::MAX`
pub(crate) fn java_int(text: &str) -> Option<i32> {
    match text.trim() {
        "Integer.MAX_VALUE" => Some(i32::MAX),
        t => t.parse().ok(),
    }
}

/// A `ManaType` argument as a pool kind (WUBRG 0-4, colorless 5)
pub(crate) fn mana_type(text: &str) -> Option<usize> {
    let kinds = ["WHITE", "BLUE", "BLACK", "RED", "GREEN", "COLORLESS"];
    kinds.iter().position(|k| text.trim().strip_prefix("ManaType.") == Some(*k))
}

/// A "counter" argument (`card_handlers.py`: "<counter name> <n>")
pub(crate) fn counter_arg(text: &str) -> Option<(&str, i32)> {
    let (name, n) = text.trim().rsplit_once(' ')?;
    Some((name, n.parse().ok()?))
}

/// `TargetSource.choose` for a resolving static's `init` (`Handler::on_start`): "a source of your choice", any
/// spell on the stack, permanent, or card in a graveyard or exile; nothing to choose, nothing chosen
pub(crate) fn choose_source(game: &mut Game, d: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let mut can: Vec<ObjId> = game.stack.iter().filter(|i| i.ability.is_none()).map(|i| i.card).collect();
    can.extend(game.battlefield.iter().copied());
    for p in 0..game.seats() {
        can.extend(game.player(p).graveyard.iter().copied());
    }
    can.extend(game.exile.iter().copied());
    can.dedup();
    if can.is_empty() {
        return Some(Vec::new());
    }
    let chosen = choose_cards(game, d, cx.controller, &can, &can, 1, 1, "source of your choice")?;
    Some(chosen.into_iter().map(|o| (Target::Obj(o), game.obj(o).zcc)).collect())
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
    /// an increase when its first constructor argument is 0 or more, else a reduction (Java picks the
    /// `CostModificationType` from the amount's sign: `SpellsCostModificationThatTargetSourceEffect`)
    BySign,
    /// `INCREASE_COST` adding `PayLifeCost(n)` to a spell (`abilityToModify.addCost`): paid after its other costs,
    /// checked once its targets are chosen (`rules::life_cost`); its `apply` isn't called
    Life(i32),
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

/// What a rule-modifying effect stops (Java's `ContinuousRuleModifyingEffect` event types: each a "can't")
#[allow(dead_code, clippy::enum_variant_names)]
pub(crate) enum Rule {
    /// `CAST_SPELL` (and `CAST_SPELL_LATE`): `seat` can't cast the card `id` as `face`
    CantCast(fn(&Game, &StaticCx, Seat, ObjId, &CardDef) -> bool),
    /// `ACTIVATE_ABILITY` of an activated ability that isn't a mana ability: `seat` can't activate the ability at
    /// `index` of `id` (Faith's Fetters)
    CantActivate(fn(&Game, &StaticCx, Seat, ObjId, usize) -> bool),
    /// `LOSES` (`false`: `seat` can't lose the game, `PlayerImpl.canLose`) and `WINS` (`true`: `seat` can't win it,
    /// `PlayerImpl.won`)
    CantLoseOrWin(fn(&Game, &StaticCx, Seat, bool) -> bool),
}

/// `YouDontLoseManaEffect`'s `addDoNotEmptyManaType`: the kinds of `seat`'s mana (WUBRG, then colorless) the pool
/// keeps as steps and phases end
pub(crate) type KeepsManaFn = fn(&Game, &StaticCx, Seat) -> [bool; 6];

/// `ContinuousEffect.init` of a resolving static: what it chose then (Java's target pointer: "a source of your
/// choice"), kept as its targets; None when the decider stopped
pub(crate) type StartFn = fn(&mut Game, &mut dyn Decider, &StaticCx) -> Option<Vec<(Target, u32)>>;

/// What an as-though effect allows
#[allow(dead_code)]
pub(crate) enum AsThough {
    /// `CAST_AS_THOUGH_IT_HAD_FLASH`: `seat` may cast the card `id` as `face` as though it had flash
    Flash(fn(&Game, &StaticCx, Seat, ObjId, &CardDef) -> bool),
    /// `ACTIVATE_HASTE`: the {T} costs of the permanent `id`'s abilities can be paid as though it had haste
    Haste(fn(&Game, &StaticCx, ObjId) -> bool),
    /// `HEXPROOF`: `seat`'s spells and abilities can target the player or permanent as though it didn't have
    /// hexproof
    Hexproof(fn(&Game, &StaticCx, Seat, Target) -> bool),
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
                        | Hook::AsThough(_)
                        | Hook::KeepsMana(_),
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
#[allow(clippy::too_many_arguments)]
fn visit(
    game: &Game,
    e: &EffectDef,
    source: ObjId,
    controller: Seat,
    timestamp: u64,
    targets: &[(Target, u32)],
    f: &mut dyn FnMut(&'static Hook, &StaticCx, &EffectDef),
) {
    match e {
        EffectDef::CardStatic { handler, args, effects, filters, abilities } => {
            let cx = StaticCx { source, controller, timestamp, args, effects, filters, abilities, targets };
            f(handler.hook(), &cx, e);
        }
        // the condition only for a handler's effect: a condition can ask the queries the handlers answer
        EffectDef::ConditionalContinuousEffect { effect, condition }
            if has_card_static(effect) && game.condition_holds(Some(condition), controller, source) =>
        {
            visit(game, effect, source, controller, timestamp, targets, f);
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
                visit(game, e, *src, o.controller, game.static_at(*src), &[], f);
            }
        }
    }
    // a static ability a static gives ("creatures and enchantments you control have 'spells your opponents cast that
    // target this permanent cost {1} more'": Callaphe), each permanent it's given to its source
    for src in game.battlefield.iter() {
        let s = game.obj(*src);
        for a in &s.def.abilities {
            let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
            for e in effects {
                let (EffectDef::GainAbilityControlledEffect { ability, .. }
                | EffectDef::GainAbilityAllEffect { ability, .. }
                | EffectDef::GainAbilityAttachedEffect { ability }) = e
                else {
                    continue;
                };
                let AbilityDef::SimpleStaticAbility { effects: given, .. } = &**ability else { continue };
                if !given.iter().any(|g| matches!(g, EffectDef::CardStatic { .. })) {
                    continue;
                }
                for id in game.battlefield.iter() {
                    if game.static_gives(e, *src, s.controller, *id, ability) {
                        let o = game.obj(*id);
                        for g in given {
                            visit(game, g, *id, o.controller, game.static_at(*src), &[], f);
                        }
                    }
                }
            }
        }
    }
    for g in game.grants.iter() {
        let AbilityDef::SimpleStaticAbility { effects, .. } = &g.ability else { continue };
        let o = game.obj(g.target);
        if o.zone == Zone::Battlefield && o.zcc == g.zcc {
            for e in effects {
                visit(game, e, g.target, o.controller, game.static_at(g.target), &[], f);
            }
        }
    }
    for r in game.card_statics.iter() {
        visit(game, &r.effect, r.source, r.controller, r.timestamp, &r.targets, f);
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
                    visit(game, e, id, o.controller, 0, &[], f);
                }
            }
        }
    }
}

/// A `CardStatic` resolving as a one-shot effect: it lasts until the turn's cleanup, pointed at the resolving
/// ability's targets (or what its `Handler::on_start` chooses). None when the decider stopped.
pub(crate) fn resolve_static(
    game: &mut Game,
    d: &mut dyn Decider,
    effect: &EffectDef,
    source: ObjId,
    controller: Seat,
    legal: &[Vec<Target>],
) -> Option<()> {
    let timestamp = game.timestamp();
    // (the first target's, as Java's target pointer)
    let mut targets: Vec<(Target, u32)> = legal
        .first()
        .into_iter()
        .flatten()
        .map(|t| match t {
            Target::Obj(o) => (*t, game.obj(*o).zcc),
            Target::Player(_) => (*t, 0),
        })
        .collect();
    let mut until_turn_of = None;
    if let EffectDef::CardStatic { handler, args, effects, filters, abilities } = effect {
        if let Some(start) = handler.0.start {
            let cx = StaticCx { source, controller, timestamp, args, effects, filters, abilities, targets: &targets };
            targets = start(game, d, &cx)?;
        }
        if args.first().is_some_and(|a| a.trim() == "Duration.UntilYourNextTurn") {
            until_turn_of = Some(controller);
        }
    }
    let r = ResolvedStatic { effect: effect.clone(), source, controller, timestamp, targets, until_turn_of };
    game.card_statics.push(r);
    Some(())
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

/// Layer 7b: the base power and toughness the handlers set on `id`, each with its timestamp (no power: only the
/// toughness is set)
pub(crate) fn set_pt(game: &Game, id: ObjId) -> Vec<(u64, Option<i32>, i32)> {
    let mut out = Vec::new();
    each_static(game, &mut |h, cx, _| match h {
        Hook::Continuous(Continuous { affects, layer: Layer::SetPt(f) }) if affects(game, cx, id) => {
            let (p, t) = f(game, cx, id);
            out.push((cx.timestamp, Some(p), t));
        }
        Hook::Continuous(Continuous { affects, layer: Layer::SetToughness(f) }) if affects(game, cx, id) => {
            out.push((cx.timestamp, None, f(game, cx, id)));
        }
        _ => {}
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

/// Whether a rule-modifying handler says `seat` can't lose the game (`LOSES`), or with `wins`, can't win it (`WINS`)
pub(crate) fn cant_lose_or_win(game: &Game, seat: Seat, wins: bool) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::CantLoseOrWin(f)) = h {
            out = out || f(game, cx, seat, wins);
        }
    });
    out
}

/// Whether a rule-modifying handler stops `seat` activating the (non-mana) ability at `index` of `id`
pub(crate) fn cant_activate(game: &Game, seat: Seat, id: ObjId, index: usize) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::CantActivate(f)) = h {
            out = out || f(game, cx, seat, id, index);
        }
    });
    out
}

/// Whether an as-though handler lets `seat` target `t` as though it didn't have hexproof
pub(crate) fn ignore_hexproof(game: &Game, seat: Seat, t: Target) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::AsThough(AsThough::Hexproof(f)) = h {
            out = out || f(game, cx, seat, t);
        }
    });
    out
}

/// Whether an as-though handler lets the {T} costs of `id`'s abilities be paid as though it had haste
pub(crate) fn activate_haste(game: &Game, id: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::AsThough(AsThough::Haste(f)) = h {
            out = out || f(game, cx, id);
        }
    });
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
            let k = match k {
                CostKind::BySign if cx.arg_int(0).unwrap_or(0) >= 0 => CostKind::Increase,
                CostKind::BySign => CostKind::Reduce,
                k => *k,
            };
            if k == kind && applies(game, cx, of) {
                apply(game, cx, of, cost);
            }
        }
    };
    each_static(game, &mut change);
    if of.ability.is_none() {
        each_own_static(game, of.card, &mut change);
    }
}

/// `ManaPool.emptyPool`'s `doNotEmptyManaTypes`: the kinds of `seat`'s mana (WUBRG, then colorless) a handler keeps
/// as steps and phases end
pub(crate) fn kept_mana_kinds(game: &Game, seat: Seat) -> [bool; 6] {
    let mut out = [false; 6];
    each_static(game, &mut |h, cx, _| {
        if let Hook::KeepsMana(f) = h {
            for (k, keep) in f(game, cx, seat).into_iter().enumerate() {
                out[k] |= keep;
            }
        }
    });
    out
}

/// The life the cost handlers of `CostKind::Life` add to a spell being cast (after its targets are chosen)
pub(crate) fn life_cost(game: &Game, of: &CostCx) -> i32 {
    let mut out = 0;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Cost(CostMod { kind: CostKind::Life(n), applies, .. }) = h {
            if applies(game, cx, of) {
                out += n;
            }
        }
    });
    out
}

/// How many replacement handlers apply to `ev` as it is (to count Java's "which first" questions with the engine's
/// own replacements of the event)
pub(crate) fn replacements_applying(game: &Game, ev: &Event) -> usize {
    let mut n = 0;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Replacement(Replacement { events, applies, .. }) = h {
            if events.contains(&ev.kind)
                && !ev.applied.contains(&cx.source)
                && !own_entering(ev, cx)
                && applies(game, cx, ev)
            {
                n += 1;
            }
        }
    });
    n
}

/// A permanent's own statics don't see it entering (Java's aren't active until it's on the battlefield)
fn own_entering(ev: &Event, cx: &StaticCx) -> bool {
    ev.kind == EventType::EntersBattlefield && ev.target == Some(cx.source)
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
    type Found = (EffectDef, ObjId, Seat, u64, Vec<(Target, u32)>);
    loop {
        let mut found: Vec<Found> = Vec::new();
        each_static(game, &mut |h, cx, e| {
            if let Hook::Replacement(Replacement { events, applies, .. }) = h {
                if events.contains(&ev.kind)
                    && !ev.applied.contains(&cx.source)
                    && !own_entering(ev, cx)
                    && applies(game, cx, ev)
                {
                    found.push((e.clone(), cx.source, cx.controller, cx.timestamp, cx.targets.to_vec()));
                }
            }
        });
        if found.is_empty() {
            return false;
        }
        if found.len() + others > 1 {
            game.owed_replacement_choices.borrow_mut().push(ev.player);
        }
        let (e, source, controller, timestamp, targets) = found.swap_remove(0);
        let EffectDef::CardStatic { handler, args, effects, filters, abilities } = &e else { unreachable!() };
        let Hook::Replacement(r) = handler.hook() else { unreachable!() };
        let cx = StaticCx { source, controller, timestamp, args, effects, filters, abilities, targets: &targets };
        ev.applied.push(source);
        if (r.replace)(game, &cx, ev) {
            return true;
        }
    }
}
