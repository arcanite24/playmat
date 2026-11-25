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
//! - **restrictions**: `Handler::restriction(name, applies, &[Restrict::…])`: `rules::can_block`, `can_attack`,
//!   `transform`.
//! - **rule-modifying**: `Handler::rule(name, Rule::CantCast(f))`: `rules::castable` (and the cast itself);
//!   `Rule::CantCastOnStack(f)`: only the cast itself, after its targets (`CAST_SPELL_LATE` reading the spell);
//!   `Rule::CantActivate(f)`: `rules::activatable` (activated abilities that aren't mana abilities);
//!   `Rule::CantLoseOrWin(f)`: the state-based loss check and `rules::win_game`;
//!   `Rule::MaxActivations(f)`: a limited activated ability's times each turn (`rules::activatable`);
//!   `Rule::CantHaveCounters(f)`: counters put on a permanent (`rules::counters_to_add`, before replacements).
//!   `Rule::KeepsDamage(f)`: the cleanup step's damage removal; `Rule::CantCrew(f)`: a crew cost's taps;
//!   `Rule::CantBeTargeted(f)`: a permanent as a target (`rules::targetable`); `Rule::Several(&[…])`: more than one;
//!   `Rule::CantSearchLibrary(f)`: a library search (`rules::search_library` finds nothing);
//!   `Rule::ManaBecomes(f)`: unspent mana changes kind instead of emptying (`rules::end_step`);
//!   `Rule::CantPlayLands(f)`: `rules::can_play_land` (`PLAY_LAND` by a player);
//!   `Rule::CantPlayLandsFromHand(f)`: `rules::playable`'s lands in hand (`PLAY_LAND` of a card in hand);
//!   `Rule::CantVenture(f)`: `rules::venture` (`VENTURE` by a player: nothing happens).
//! - **as-though**: `Handler::as_though(name, AsThough::Flash(f))`: `rules::castable`'s timing;
//!   `AsThough::Haste(f)`: a {T} cost's summoning sickness (`rules::can_pay_cost`, mana abilities);
//!   `AsThough::BlockDragon(f)`: blocking a Dragon with flying (`rules::can_block`);
//!   `AsThough::ActivateAsInstant(f)`: a sorcery-speed activated ability's timing (`rules::activatable`);
//!   `AsThough::Hexproof(f)`: hexproof, as targets are chosen (`rules::possible_targets`, `targetable`);
//!   `AsThough::DamageNotBlocked(f)`: a blocked attacker assigns its combat damage as though it weren't blocked
//!   (`rules::combat_damage`).
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
    /// layer 7b, the base power alone (`SetBasePowerEnchantedEffect`: Withercrown)
    SetPower(fn(&Game, &StaticCx, ObjId) -> i32),
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
            Layer::SetPt(_) | Layer::SetToughness(_) | Layer::SetPower(_) => ("PTChangingEffects_7", "SetPT_7b"),
            Layer::ModifyPt(_) => ("PTChangingEffects_7", "ModifyPT_7c"),
        }
    }
}

pub(crate) struct Continuous {
    pub affects: AffectsFn,
    pub layer: Layer,
    /// the class's other layers, for the same objects (`Handler::and_layers`: Sigarda's Summons is a 4, a 6 and a 7b
    /// effect)
    pub also: &'static [Layer],
}

impl Continuous {
    /// each layer it works in
    pub fn layers(&self) -> impl Iterator<Item = &Layer> {
        std::iter::once(&self.layer).chain(self.also.iter())
    }
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
    /// `CONNIVE`: `target`, controlled by `player`, would connive `amount` (`ConniveSourceEffect.connive`); "replaced"
    /// stops it, else it connives as it was going to (Leader, Super-Genius draws first)
    Connive,
    /// `ADD_COUNTERS`: `amount` `counter` counters would be put on the permanent `target` by `player` (who puts them,
    /// else its controller) (`rules::add_counters_by`, `put_counters`, a cost's counters). Change `amount` to put
    /// that many instead (Prairie Dog); "replaced" puts none
    AddCounters,
    /// `TRANSFORMING`: the permanent `target`, controlled by `player`, has just turned to its other face, whose
    /// statics already apply (`PermanentImpl.transform`, before `TRANSFORMED`). Nothing to change: a replacement
    /// does something as it transforms ("as this creature transforms into …": Sephiroth's emblem)
    Transforming,
    /// `LOSES`: `player` would lose the game (the state-based check, `PlayerImpl.lost` through `canLose`); "replaced"
    /// and they don't (Exquisite Archangel). Not when another player wins (`lostForced`)
    Loses,
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
    /// `AddCounters`: the counter's name (`getData()`)
    pub counter: Option<String>,
    /// `TappedForMana`: the mana it produces (`ManaEvent.getMana()`), which a `replace` may change (Virtue of
    /// Strength's three times as much); `colorless` wins over it
    pub mana: Option<crate::card::Mana>,
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
            counter: None,
            mana: None,
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
    /// `REDUCE_COST` that asks first (`MayPay2LifeForColorAbility`): as a spell is cast, after the other reductions,
    /// its caster may pay `n` life ("Pay 2 life to reduce the cost by {G}?", asked in `may_pay_life`), and `apply`
    /// is made only if they did; while checking what's playable it applies whenever the life could be paid
    /// (`inCheckPlayableState`)
    MayPayLife(i32),
    /// `SET_COST` that asks first (Bruenor Battlehammer's free equip): as an ability is activated, once its targets
    /// are chosen and its other cost changes made, its controller is asked "Pay {0} to equip instead {cost}?"
    /// (`may_free`), and a yes clears its mana cost; while checking what's playable it applies (`apply` there)
    MayFree,
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
    /// `canTransform` is false (`PermanentImpl.checkTransformRestrictionEffects`)
    CantTransform(fn(&Game, &StaticCx, ObjId) -> bool),
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
    /// `CAST_SPELL_LATE` whose `applies` reads the spell on the stack (Brisela's `getStack().getSpell`): `seat` can't
    /// cast the spell `id` as `face`. The playable check finds no spell, so it holds only as the spell is cast, after
    /// its modes and targets (`AbilityImpl.activate`, 601.2e), which undoes the cast
    CantCastOnStack(fn(&Game, &StaticCx, Seat, ObjId, &CardDef) -> bool),
    /// `ACTIVATE_ABILITY` of an activated ability that isn't a mana ability: `seat` can't activate the ability at
    /// `index` of `id` (Faith's Fetters)
    CantActivate(fn(&Game, &StaticCx, Seat, ObjId, usize) -> bool),
    /// `LOSES` (`false`: `seat` can't lose the game, `PlayerImpl.canLose`) and `WINS` (`true`: `seat` can't win it,
    /// `PlayerImpl.won`)
    CantLoseOrWin(fn(&Game, &StaticCx, Seat, bool) -> bool),
    /// `MAX_ACTIVATIONS`: how many times `seat` may activate the ability at `index` of `id` each turn, where it
    /// limits them (Birgi's boast twice); None leaves the limit as it is
    MaxActivations(fn(&Game, &StaticCx, Seat, ObjId, usize) -> Option<u32>),
    /// `CAN_ADD_COUNTERS`: the permanent `id` can't have counters named so put on it (`PermanentImpl.addCounters`
    /// asks before any `ADD_COUNTERS` replacement: none of them is applied or asked about)
    CantHaveCounters(fn(&Game, &StaticCx, ObjId, &str) -> bool),
    /// `REMOVE_DAMAGE_EOT`: the damage marked on the permanent `id` isn't removed in the cleanup step
    /// (`PermanentImpl.endOfTurn`: Ancient Adamantoise)
    KeepsDamage(fn(&Game, &StaticCx, ObjId) -> bool),
    /// `CREW_VEHICLE`: the creature `id`, chosen to crew a Vehicle, isn't tapped for it and adds no power
    /// (`CrewAbility`'s cost: Intercessor's Arrest)
    CantCrew(fn(&Game, &StaticCx, ObjId) -> bool),
    /// `TARGET`: `seat`'s spell or ability whose source object is `src` (a spell on the stack, or a permanent's or
    /// card's ability) can't target the permanent `id` (`PermanentImpl.canBeTargetedBy`, after protection)
    CantBeTargeted(fn(&Game, &StaticCx, Seat, ObjId, ObjId) -> bool),
    /// one class stopping several events (`checksEventType` with two or more: Intercessor's Arrest's `CREW_VEHICLE`
    /// and `ACTIVATE_ABILITY`): each of these, read as if it were its own handler
    Several(&'static [Rule]),
    /// `SEARCH_LIBRARY`: `seat` can't search the library of `owner` for the spell or ability of `source`
    /// (`PlayerImpl.searchLibrary` returns false: nothing is found, the effect goes on)
    CantSearchLibrary(fn(&Game, &StaticCx, Seat, Seat, ObjId) -> bool),
    /// `ManaPool.setManaBecomesBlack` / `Red` / `Colorless` (a `RulesEffects` continuous effect): the mana `seat`
    /// would lose as steps and phases end becomes this kind instead (WUBRG 0-4, colorless 5); None leaves it
    ManaBecomes(fn(&Game, &StaticCx, Seat) -> Option<usize>),
    /// `PLAY_LAND` whose `applies` reads only the player: `seat` can't play lands (Aggressive Mining), read by
    /// `rules::can_play_land`
    CantPlayLands(fn(&Game, &StaticCx, Seat) -> bool),
    /// `PLAY_LAND` of a land card in its player's hand (`getZone(event.getSourceId()) == Zone.HAND`): `seat` can't
    /// play lands from their hand (Experimental Frenzy), read where `rules::playable` offers the hand's lands
    CantPlayLandsFromHand(fn(&Game, &StaticCx, Seat) -> bool),
    /// `VENTURE` whose `applies` reads only the player: `seat` doesn't venture into the dungeon
    /// (`GameImpl.ventureIntoDungeon` returns: Keen-Eared Sentry)
    CantVenture(fn(&Game, &StaticCx, Seat) -> bool),
    /// `UNTAP` replaced with nothing (a `ReplacementEffect` whose `replaceEvent` returns true: Blossombind, Frozen in
    /// Ice): the permanent `id` can't become untapped, in its untap step or by an effect (`PermanentImpl.untap`)
    CantUntap(fn(&Game, &StaticCx, ObjId) -> bool),
    /// `SEARCH_LIBRARY` whose `replaceEvent` sets its amount (Aven Mindcensor): `seat` searching `owner`'s library
    /// searches only that many cards from its top (`PlayerImpl.searchLibrary`'s `librarySearchLimit`); None leaves it
    SearchLimit(fn(&Game, &StaticCx, Seat, Seat) -> Option<usize>),
    /// a `Layer.PlayerEffects` continuous effect's `player.setMaxHandSize`: `seat`'s maximum hand size, where it
    /// sets one (Winter, Misanthropic Guide); None leaves it. Read by `rules::max_hand_size`, after the engine's own.
    MaxHandSize(fn(&Game, &StaticCx, Seat) -> Option<usize>),
    /// `GameState.addPowerInsteadOfToughnessForDamageLethalityFilter`: lethal damage to the creature `id` is
    /// determined by its power rather than its toughness (Zilortha, Strength Incarnate), read by the state-based
    /// actions (`max(power, 1)`) and `Permanent.getLethalDamage` (power)
    LethalByPower(fn(&Game, &StaticCx, ObjId) -> bool),
}

/// The rules a rule-modifying handler's hook has (`Rule::Several`'s, or its one); none for another kind
fn rule_parts(h: &'static Hook) -> &'static [Rule] {
    match h {
        Hook::Rule(Rule::Several(rs)) => rs,
        Hook::Rule(r) => std::slice::from_ref(r),
        _ => &[],
    }
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
    /// `PLAY_FROM_NOT_OWN_HAND_ZONE` / `CAST_FROM_NOT_OWN_HAND_ZONE`: `seat` may play the card `id` as `face` from
    /// the graveyard or exile it's in; Some: how (`PlayWay`). A card's own (`Zone.ALL`: "you may cast this card from
    /// your graveyard") is declared `.in_any_zone()`.
    Play(fn(&Game, &StaticCx, Seat, ObjId, &CardDef) -> Option<PlayWay>),
    /// `Play`, and what casting a spell that way does once it's cast (`setCastSourceIdWithAlternateMana`'s
    /// identifier, read by the card's watcher on `SPELL_CAST`: "that creature enters with a finality counter on it")
    PlayThen(fn(&Game, &StaticCx, Seat, ObjId, &CardDef) -> Option<PlayWay>, CastThisWayFn),
    /// `BLOCK_DRAGON`: the creature `id` can block Dragons with flying as though it had reach (`FlyingAbility`)
    BlockDragon(fn(&Game, &StaticCx, ObjId) -> bool),
    /// `ACTIVATE_AS_INSTANT`: `seat` may activate the ability at `index` of `id` any time they could cast an
    /// instant (Teferi, Master of Time's loyalty abilities)
    ActivateAsInstant(fn(&Game, &StaticCx, Seat, ObjId, usize) -> bool),
    /// `DAMAGE_NOT_BLOCKED`: the blocked attacker `id` may assign its combat damage as though it weren't blocked.
    /// None: it doesn't apply; Some(false): it applies; Some(true): it applies if the effect's controller says yes
    /// to "Have (the attacker) assign damage as though it weren't blocked?" (an `applies` that asks: Zilortha's)
    DamageNotBlocked(fn(&Game, &StaticCx, ObjId) -> Option<bool>),
}

/// What casting a spell by an `AsThough::PlayThen` way does: (the permitting source, its controller, the spell).
/// None when the decider stopped.
pub(crate) type CastThisWayFn = fn(&mut Game, &mut dyn Decider, ObjId, Seat, ObjId) -> Option<()>;

/// How an as-though effect lets a card be played from a graveyard or exile: for its own mana cost, or for `mana`
/// instead (`setCastSourceIdWithAlternateMana`), with `costs` paid besides
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayWay {
    pub mana: Option<ManaCost>,
    pub costs: Vec<crate::card::CostDef>,
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
        if handler.0.until_your_next_turn || args.first().is_some_and(|a| a.trim() == "Duration.UntilYourNextTurn") {
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
        let Hook::Continuous(c) = h else { return };
        for layer in c.layers() {
            if let Layer::ModifyPt(f) = layer {
                if (c.affects)(game, cx, id) {
                    let (p, t) = f(game, cx, id);
                    out.0 += p;
                    out.1 += t;
                }
            }
        }
    });
    out
}

/// Layer 7b: the base power and toughness the handlers set on `id`, each with its timestamp (no power: only the
/// toughness is set; no toughness: only the power)
pub(crate) fn set_pt(game: &Game, id: ObjId) -> Vec<(u64, Option<i32>, Option<i32>)> {
    let mut out = Vec::new();
    each_static(game, &mut |h, cx, _| {
        let Hook::Continuous(c) = h else { return };
        for layer in c.layers() {
            match layer {
                Layer::SetPt(f) if (c.affects)(game, cx, id) => {
                    let (p, t) = f(game, cx, id);
                    out.push((cx.timestamp, Some(p), Some(t)));
                }
                Layer::SetToughness(f) if (c.affects)(game, cx, id) => {
                    out.push((cx.timestamp, None, Some(f(game, cx, id))));
                }
                Layer::SetPower(f) if (c.affects)(game, cx, id) => {
                    out.push((cx.timestamp, Some(f(game, cx, id)), None));
                }
                _ => {}
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
        let Hook::Continuous(c) = h else { return };
        for layer in c.layers() {
            if let Layer::Ability(f) = layer {
                if !lost_at.is_some_and(|at| cx.timestamp < at) && (c.affects)(game, cx, id) {
                    out.extend(f(game, cx, id));
                }
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
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        let Hook::Continuous(c) = h else { return };
        for layer in c.layers() {
            if let Layer::Type(f) = layer {
                if !out && (c.affects)(game, cx, id) && f(game, cx, id).iter().any(|x| x == s) {
                    out = true;
                }
            }
        }
    };
    each_static(game, &mut check);
    // a card's own, in every zone (`Zone.ALL`: `HasSubtypesSourceEffect`)
    each_own_static(game, id, &mut check);
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
                Restrict::CantAttack(_) | Restrict::CantTransform(_) => false,
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

/// Whether a handler's restriction says the permanent `id` can't transform
pub(crate) fn cant_transform(game: &Game, id: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if out {
            return;
        }
        if let Hook::Restriction(Restriction { applies, restrict }) = h {
            out = restrict
                .iter()
                .any(|r| matches!(r, Restrict::CantTransform(f) if applies(game, cx, id) && f(game, cx, id)));
        }
    });
    out
}

/// Whether a rule-modifying handler stops `seat` casting `id` as `face` (the card's own too: "you can't cast this
/// spell unless …")
pub(crate) fn cant_cast(game: &Game, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    let mut out = false;
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        for r in rule_parts(h) {
            if let Rule::CantCast(f) = r {
                out = out || f(game, cx, seat, id, face);
            }
        }
    };
    each_static(game, &mut check);
    each_own_static(game, id, &mut check);
    out
}

/// Whether a rule-modifying handler stops `seat` casting `id`, on the stack now as `face`, once its modes and
/// targets are chosen
pub(crate) fn cant_cast_on_stack(game: &Game, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    let mut out = false;
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        for r in rule_parts(h) {
            if let Rule::CantCastOnStack(f) = r {
                out = out || f(game, cx, seat, id, face);
            }
        }
    };
    each_static(game, &mut check);
    // the spell's own (`Zone.ALL`: `CastAsThoughItHadFlashIfConditionAbility`'s "can't flash unless")
    each_own_static(game, id, &mut check);
    out
}

/// Whether a rule-modifying handler says the permanent `id` can't have `counter` counters put on it
pub(crate) fn cant_have_counters(game: &Game, id: ObjId, counter: &str) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::CantHaveCounters(f)) = h {
            out = out || f(game, cx, id, counter);
        }
    });
    out
}

/// How many cards from the top of `owner`'s library `seat` searches, if a handler limits it (the last one that
/// applies sets the event's amount, `SearchLibraryEvent.setAmount`)
pub(crate) fn search_limit(game: &Game, seat: Seat, owner: Seat) -> Option<usize> {
    let mut out = None;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::SearchLimit(f)) = h {
            if let Some(n) = f(game, cx, seat, owner) {
                out = Some(n);
            }
        }
    });
    out
}

/// Whether a handler says the permanent `id` can't become untapped (`UNTAP` replaced)
pub(crate) fn cant_untap(game: &Game, id: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::CantUntap(f)) = h {
            out = out || f(game, cx, id);
        }
    });
    out
}

/// Whether a rule-modifying handler says `seat` can't lose the game (`LOSES`), or with `wins`, can't win it (`WINS`)
pub(crate) fn cant_lose_or_win(game: &Game, seat: Seat, wins: bool) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::CantLoseOrWin(f) = r {
                out = out || f(game, cx, seat, wins);
            }
        }
    });
    out
}

/// Whether a rule-modifying handler stops `seat`'s spell or ability from `src` targeting the permanent `id`
/// (`TARGET`)
pub(crate) fn cant_be_targeted(game: &Game, seat: Seat, id: ObjId, src: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::CantBeTargeted(f) = r {
                out = out || f(game, cx, seat, id, src);
            }
        }
    });
    out
}

/// Whether a rule-modifying handler stops `id` crewing a Vehicle (`CREW_VEHICLE`)
pub(crate) fn cant_crew(game: &Game, id: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::CantCrew(f) = r {
                out = out || f(game, cx, id);
            }
        }
    });
    out
}

/// Whether a rule-modifying handler keeps the damage on `id` through the cleanup step (`REMOVE_DAMAGE_EOT`)
pub(crate) fn keeps_damage(game: &Game, id: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::KeepsDamage(f) = r {
                out = out || f(game, cx, id);
            }
        }
    });
    out
}

/// Whether a rule-modifying handler says `seat` can't venture into the dungeon (`VENTURE`)
pub(crate) fn cant_venture(game: &Game, seat: Seat) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::CantVenture(f) = r {
                out = out || f(game, cx, seat);
            }
        }
    });
    out
}

/// The maximum hand size a handler sets for `seat` (the last one, in the order the statics are read), if any
pub(crate) fn handler_max_hand_size(game: &Game, seat: Seat) -> Option<usize> {
    let mut out = None;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::MaxHandSize(f) = r {
                if let Some(n) = f(game, cx, seat) {
                    out = Some(n);
                }
            }
        }
    });
    out
}

/// Whether lethal damage to the creature `id` is determined by its power (`Rule::LethalByPower`)
pub(crate) fn lethal_by_power(game: &Game, id: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::LethalByPower(f) = r {
                out = out || f(game, cx, id);
            }
        }
    });
    out
}

/// The power or toughness lethal damage to `id` is measured against (`Permanent.getLethalDamage`)
pub(crate) fn lethal_measure(game: &Game, id: ObjId) -> i32 {
    let (p, t) = game.power_toughness(id);
    if lethal_by_power(game, id) {
        p
    } else {
        t
    }
}

/// Whether a rule-modifying handler says `seat` can't play lands (`PLAY_LAND`)
pub(crate) fn cant_play_lands(game: &Game, seat: Seat) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::CantPlayLands(f)) = h {
            out = out || f(game, cx, seat);
        }
    });
    out
}

/// Whether a rule-modifying handler says `seat` can't play lands from their hand
pub(crate) fn cant_play_lands_from_hand(game: &Game, seat: Seat) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::CantPlayLandsFromHand(f) = r {
                out = out || f(game, cx, seat);
            }
        }
    });
    out
}

/// Whether a rule-modifying handler stops `seat` activating the (non-mana) ability at `index` of `id`
pub(crate) fn cant_activate(game: &Game, seat: Seat, id: ObjId, index: usize) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::CantActivate(f) = r {
                out = out || f(game, cx, seat, id, index);
            }
        }
    });
    out
}

/// Whether a rule-modifying handler stops `seat` searching `owner`'s library for `source`'s spell or ability
pub(crate) fn cant_search_library(game: &Game, seat: Seat, owner: Seat, source: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::CantSearchLibrary(f)) = h {
            out = out || f(game, cx, seat, owner, source);
        }
    });
    out
}

/// The kind `seat`'s unspent mana becomes instead of emptying, if a handler says so (`ManaPool.emptyItem` checks
/// black, then red, then colorless)
pub(crate) fn mana_becomes(game: &Game, seat: Seat) -> Option<usize> {
    let mut kinds = [false; 6];
    each_static(game, &mut |h, cx, _| {
        if let Hook::Rule(Rule::ManaBecomes(f)) = h {
            if let Some(k) = f(game, cx, seat) {
                kinds[k] = true;
            }
        }
    });
    [2, 3, 5].into_iter().find(|k| kinds[*k])
}

/// The most times `seat` may activate the ability at `index` of `id` this turn: `limit`, unless a handler changes it
/// (`MAX_ACTIVATIONS`: the last one that applies sets it)
pub(crate) fn max_activations(game: &Game, seat: Seat, id: ObjId, index: usize, limit: u32) -> u32 {
    let mut out = limit;
    each_static(game, &mut |h, cx, _| {
        for r in rule_parts(h) {
            if let Rule::MaxActivations(f) = r {
                if let Some(n) = f(game, cx, seat, id, index) {
                    out = n;
                }
            }
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

/// Whether an as-though handler lets `id` block Dragons as though it had reach (`BLOCK_DRAGON`)
pub(crate) fn block_dragon(game: &Game, id: ObjId) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::AsThough(AsThough::BlockDragon(f)) = h {
            out = out || f(game, cx, id);
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

/// The as-though handlers that let `seat` play `id` as `face` from where it is (a graveyard or exile): each source
/// with the way it allows, the statics in play first, then the card's own
pub(crate) fn play_ways(game: &Game, seat: Seat, id: ObjId, face: &CardDef) -> Vec<(ObjId, PlayWay)> {
    let mut out: Vec<(ObjId, PlayWay)> = Vec::new();
    if !matches!(game.obj(id).zone, Zone::Graveyard | Zone::Exile) {
        return out;
    }
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        if let Hook::AsThough(AsThough::Play(f) | AsThough::PlayThen(f, _)) = h {
            if let Some(way) = f(game, cx, seat, id, face) {
                if !out.iter().any(|(s, _)| *s == cx.source) {
                    out.push((cx.source, way));
                }
            }
        }
    };
    each_static(game, &mut check);
    each_own_static(game, id, &mut check);
    out
}

/// A spell `seat` cast by `source`'s as-though way (`AsThough::PlayThen`): what casting it that way does, for each
/// such handler of the source's. None when the decider stopped.
pub(crate) fn cast_this_way(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    source: ObjId,
    spell: ObjId,
) -> Option<()> {
    let mut then: Vec<(CastThisWayFn, Seat)> = Vec::new();
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        if let Hook::AsThough(AsThough::PlayThen(_, f)) = h {
            if cx.source == source && cx.controller == seat {
                then.push((*f, cx.controller));
            }
        }
    };
    each_static(game, &mut check);
    each_own_static(game, spell, &mut check);
    for (f, controller) in then {
        f(game, d, source, controller, spell)?;
    }
    Some(())
}

/// Whether an as-though handler lets `seat` activate the ability at `index` of `id` any time they could cast an
/// instant
pub(crate) fn activate_as_instant(game: &Game, seat: Seat, id: ObjId, index: usize) -> bool {
    let mut out = false;
    each_static(game, &mut |h, cx, _| {
        if let Hook::AsThough(AsThough::ActivateAsInstant(f)) = h {
            out = out || f(game, cx, seat, id, index);
        }
    });
    out
}

/// The as-though handlers that apply to the blocked attacker `id` assigning its combat damage as though it weren't
/// blocked (`AsThough::DamageNotBlocked`), in the order they're found: each one's controller, and whether it asks
pub(crate) fn damage_not_blocked(game: &Game, id: ObjId) -> Vec<(Seat, bool)> {
    let mut out = Vec::new();
    let mut check = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        if let Hook::AsThough(AsThough::DamageNotBlocked(f)) = h {
            if let Some(asks) = f(game, cx, id) {
                out.push((cx.controller, asks));
            }
        }
    };
    each_static(game, &mut check);
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

/// `CostKind::MayFree` as an ability is activated: for each handler that applies, in order, "Pay {0} to equip
/// instead {cost}?"; a yes clears the mana cost. False when the decider stopped.
pub(crate) fn may_free(game: &mut Game, d: &mut dyn Decider, of: &CostCx, mana: &mut ManaCost) -> bool {
    let mut asks = 0;
    each_static(game, &mut |h, cx, _| {
        if let Hook::Cost(CostMod { kind: CostKind::MayFree, applies, .. }) = h {
            if applies(game, cx, of) {
                asks += 1;
            }
        }
    });
    for _ in 0..asks {
        let q = Query::YesNo { message: format!("Pay {{0}} to equip instead {}?", mana_text(mana)) };
        let yes = loop {
            match ask(game, d, of.seat, &q) {
                Answer::Bool(b) => break b,
                Answer::Stop => return false,
                _ => {}
            }
        };
        if yes {
            *mana = ManaCost::default();
        }
    }
    true
}

/// `ManaCosts.getText` of what's left to pay: generic, then the colored symbols in WUBRG order, then colorless
fn mana_text(m: &ManaCost) -> String {
    let mut out = String::new();
    if m.generic > 0 {
        out += &format!("{{{}}}", m.generic);
    }
    for (k, c) in ["W", "U", "B", "R", "G"].iter().enumerate() {
        for _ in 0..m.colored[k] {
            out += &format!("{{{c}}}");
        }
    }
    for _ in 0..m.colorless {
        out += "{C}";
    }
    if out.is_empty() {
        out = "{0}".into();
    }
    out
}

/// What a spell being cast carries once its caster paid the life of a `CostKind::MayPayLife` handler
fn paid_life_mark(cx: &StaticCx) -> String {
    format!("paid life {} {}", cx.source.0, cx.timestamp)
}

/// The `CostKind::MayPayLife` reductions of a spell, after the other reductions: as it's cast (on the stack) those
/// whose life its caster paid, elsewhere (the playable check) those whose life could be paid
pub(crate) fn paid_life_reductions(game: &Game, of: &CostCx, cost: &mut ManaCost) {
    if of.ability.is_some() {
        return;
    }
    each_static(game, &mut |h, cx, _| {
        if let Hook::Cost(CostMod { kind: CostKind::MayPayLife(n), applies, apply }) = h {
            let o = game.obj(of.card);
            if applies(game, cx, of)
                && if o.zone == Zone::Stack {
                    o.marks.contains(&paid_life_mark(cx))
                } else {
                    game.player(of.seat).life >= *n
                }
            {
                apply(game, cx, of, cost);
            }
        }
    });
}

/// `MayPay2LifeEffect.apply` as the spell is cast: for each `CostKind::MayPayLife` handler that applies, in order,
/// when the life can be paid, "Pay 2 life to reduce the cost by {G}?" (the symbol its `apply` takes off), and the
/// life paid then. False when the decider stopped.
pub(crate) fn may_pay_life(game: &mut Game, d: &mut dyn Decider, of: &CostCx) -> bool {
    let mut asks: Vec<(String, i32, String)> = Vec::new();
    each_static(game, &mut |h, cx, _| {
        if let Hook::Cost(CostMod { kind: CostKind::MayPayLife(n), applies, apply }) = h {
            if applies(game, cx, of) {
                let mut probe = ManaCost { colored: [1; 5], ..ManaCost::default() };
                apply(game, cx, of, &mut probe);
                let symbol: String =
                    (0..5).filter(|k| probe.colored[*k] == 0).map(|k| ["{W}", "{U}", "{B}", "{R}", "{G}"][k]).collect();
                asks.push((paid_life_mark(cx), *n, symbol));
            }
        }
    });
    for (mark, n, symbol) in asks {
        if game.player(of.seat).life < n {
            continue;
        }
        let q = Query::YesNo { message: format!("Pay {n} life to reduce the cost by {symbol}?") };
        let yes = loop {
            match ask(game, d, of.seat, &q) {
                Answer::Bool(b) => break b,
                Answer::Stop => return false,
                _ => {}
            }
        };
        if yes && pay_cost(game, d, of.seat, of.card, &crate::card::CostDef::PayLifeCost { amount: n }) {
            game.obj_mut(of.card).marks.push(mark);
        }
        if d.stopped() {
            return false;
        }
    }
    true
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

/// The static handlers of permanents that left the battlefield in the current action (`game.lookback`), for an event
/// of something leaving the battlefield: Java's static abilities look back in time through its short-living LKI
/// (`AbilityImpl.isInUseableZone`: Valentin blocking a creature, both dying, still exiles it)
fn each_static_left(
    game: &Game,
    ev_kind: EventType,
    from: Option<Zone>,
    f: &mut dyn FnMut(&'static Hook, &StaticCx, &EffectDef),
) {
    if ev_kind != EventType::ZoneChange || from != Some(Zone::Battlefield) {
        return;
    }
    for src in game.lookback.iter() {
        if game.battlefield.contains(src) {
            continue;
        }
        let o = game.obj(*src);
        for a in &o.def.abilities {
            let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
            for e in effects {
                visit(game, e, *src, o.controller, game.static_at(*src), &[], f);
            }
        }
    }
}

/// How many replacement handlers apply to `ev` as it is (to count Java's "which first" questions with the engine's
/// own replacements of the event)
pub(crate) fn replacements_applying(game: &Game, ev: &Event) -> usize {
    let mut n = 0;
    let mut count = |h: &'static Hook, cx: &StaticCx, _: &EffectDef| {
        if let Hook::Replacement(Replacement { events, applies, .. }) = h {
            if events.contains(&ev.kind)
                && !ev.applied.contains(&cx.source)
                && !own_entering(ev, cx)
                && applies(game, cx, ev)
            {
                n += 1;
            }
        }
    };
    each_static(game, &mut count);
    each_static_left(game, ev.kind, ev.from, &mut count);
    n
}

/// A permanent's own statics don't see it entering (Java's aren't active until it's on the battlefield). An effect
/// that resolved pointed at the entering card isn't one of them, even with that card as its source: the watcher's
/// `AddCounterEnteringCreatureEffect` of a spell Osteomancer Adept let you cast has the spell's ability as its source.
fn own_entering(ev: &Event, cx: &StaticCx) -> bool {
    ev.kind == EventType::EntersBattlefield
        && ev.target == Some(cx.source)
        && !cx.targets.iter().any(|(t, _)| *t == Target::Obj(cx.source))
}

/// Whether a replacement handler for events of `kind` is in play
pub(crate) fn any_replacement(game: &Game, kind: EventType) -> bool {
    let mut out = false;
    let mut any = |h: &'static Hook, _: &StaticCx, _: &EffectDef| {
        out = out || matches!(h, Hook::Replacement(Replacement { events, .. }) if events.contains(&kind));
    };
    each_static(game, &mut any);
    each_static_left(game, kind, Some(Zone::Battlefield), &mut any);
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
        let mut find = |h: &'static Hook, cx: &StaticCx, e: &EffectDef| {
            if let Hook::Replacement(Replacement { events, applies, .. }) = h {
                if events.contains(&ev.kind)
                    && !ev.applied.contains(&cx.source)
                    && !own_entering(ev, cx)
                    && applies(game, cx, ev)
                {
                    found.push((e.clone(), cx.source, cx.controller, cx.timestamp, cx.targets.to_vec()));
                }
            }
        };
        each_static(game, &mut find);
        each_static_left(game, ev.kind, ev.from, &mut find);
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
