//! The game state. Each part sits behind an `Arc` and is copied only when changed (`Arc::make_mut`), so copying a
//! `Game` for an AI search node bumps a few reference counts.

use crate::card::{AbilityDef, Amount, CardDef, CardType, Condition, EffectDef, Filter, Mana, Pred, Trigger, Value};
use crate::rng::JavaRandom;
use std::sync::Arc;

pub type Seat = usize;

/// An object's number, given in the order objects are created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    /// emblems
    Command,
}

/// The steps of a turn, named as Java's `PhaseStep`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    FirstCombatDamage,
    CombatDamage,
    EndCombat,
    PostcombatMain,
    EndTurn,
    Cleanup,
}

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Untap => "UNTAP",
            Step::Upkeep => "UPKEEP",
            Step::Draw => "DRAW",
            Step::PrecombatMain => "PRECOMBAT_MAIN",
            Step::BeginCombat => "BEGIN_COMBAT",
            Step::DeclareAttackers => "DECLARE_ATTACKERS",
            Step::DeclareBlockers => "DECLARE_BLOCKERS",
            Step::FirstCombatDamage => "FIRST_COMBAT_DAMAGE",
            Step::CombatDamage => "COMBAT_DAMAGE",
            Step::EndCombat => "END_COMBAT",
            Step::PostcombatMain => "POSTCOMBAT_MAIN",
            Step::EndTurn => "END_TURN",
            Step::Cleanup => "CLEANUP",
        }
    }

    pub fn parse(name: &str) -> Option<Step> {
        Some(match name {
            "UNTAP" => Step::Untap,
            "UPKEEP" => Step::Upkeep,
            "DRAW" => Step::Draw,
            "PRECOMBAT_MAIN" => Step::PrecombatMain,
            "BEGIN_COMBAT" => Step::BeginCombat,
            "DECLARE_ATTACKERS" => Step::DeclareAttackers,
            "DECLARE_BLOCKERS" => Step::DeclareBlockers,
            "FIRST_COMBAT_DAMAGE" => Step::FirstCombatDamage,
            "COMBAT_DAMAGE" => Step::CombatDamage,
            "END_COMBAT" => Step::EndCombat,
            "POSTCOMBAT_MAIN" => Step::PostcombatMain,
            "END_TURN" => Step::EndTurn,
            "CLEANUP" => Step::Cleanup,
            _ => return None,
        })
    }

    pub fn is_main(self) -> bool {
        matches!(self, Step::PrecombatMain | Step::PostcombatMain)
    }
}

/// A card in some zone, or a permanent.
#[derive(Clone, Debug)]
pub struct Obj {
    pub id: ObjId,
    /// zone changes so far: an object that changes zones is a new object (rule 400.7), which targets and effects
    /// pointing at the old one don't follow
    pub zcc: u32,
    pub def: Arc<CardDef>,
    pub owner: Seat,
    pub controller: Seat,
    pub zone: Zone,
    pub tapped: bool,
    pub damage: i32,
    pub counters: Vec<(String, i32)>,
    pub face_down: bool,
    /// the player it attacks
    pub attacking: Option<Seat>,
    /// the planeswalker it attacks (and its zone change count), if it attacks one
    pub attacking_pw: Option<(ObjId, u32)>,
    /// a loyalty ability of it was activated this turn
    pub loyalty_used: bool,
    /// it entered the battlefield as a kicked spell
    pub kicked: bool,
    /// what it was before an effect changed its characteristics (`def`) until end of turn
    pub until_end_of_turn_def: Option<Arc<CardDef>>,
    /// the permanent (and its zone change count) this Aura is attached to
    pub attached_to: Option<(ObjId, u32)>,
    /// the attackers it blocks
    pub blocking: Vec<ObjId>,
    /// blocked this combat (stays blocked if its blockers leave)
    pub blocked: bool,
    /// an attacker's blockers, in the order they were declared (Java's `CombatGroup.blockers`)
    pub blocked_by: Vec<ObjId>,
    /// came under its controller's control since their most recent turn began (summoning sickness)
    pub sick: bool,
    /// dealt damage by a source with deathtouch since the last check
    pub deathtouched: bool,
    /// a token: it exists only on the battlefield
    pub token: bool,
    /// where its triggered abilities sit in Java's `TriggeredAbilities` map (registered as cards are loaded, tokens as
    /// they're created): abilities that trigger on the same event wait in this order
    pub trigger_order: u64,
}

impl Obj {
    pub fn name(&self) -> &str {
        &self.def.name
    }

    pub fn is(&self, t: CardType) -> bool {
        self.def.is(t)
    }

    pub fn has(&self, ability: &AbilityDef) -> bool {
        self.def.has(ability)
    }

    pub fn counter(&self, name: &str) -> i32 {
        self.counters.iter().filter(|(n, _)| n == name).map(|(_, k)| *k).sum()
    }

    pub fn add_counters(&mut self, name: &str, amount: i32) {
        match self.counters.iter_mut().find(|(n, _)| n == name) {
            Some(c) => c.1 += amount,
            None => self.counters.push((name.to_string(), amount)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Player {
    pub life: i32,
    pub counters: Vec<(String, i32)>,
    pub hand: Vec<ObjId>,
    /// top first
    pub library: Vec<ObjId>,
    pub graveyard: Vec<ObjId>,
    pub lost: bool,
    pub won: bool,
    pub lands_played: u32,
    pub mana_pool: Mana,
    pub passed: bool,
    /// drew from an empty library since the last state-based check
    pub drew_from_empty: bool,
    /// declared an attacker this turn (`PlayerAttackedWatcher`)
    pub attacked: bool,
    /// life lost this turn
    pub life_lost: i32,
    /// times they gained life this turn
    pub life_gains: u32,
    /// spells they cast this turn, and how many of them were instants or sorceries
    pub spells_cast: u32,
    pub instants_sorceries_cast: u32,
    /// a permanent they controlled left the battlefield this turn (`RevoltWatcher`)
    pub permanent_left: bool,
}

/// What a target is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Player(Seat),
    Obj(ObjId),
}

#[derive(Clone, Debug)]
pub struct StackItem {
    /// the spell's card, or the source of an ability
    pub card: ObjId,
    pub controller: Seat,
    /// the chosen targets, per target of the spell or ability (a target can take several, "up to two")
    pub targets: Vec<Vec<Target>>,
    /// each object target's zone-change count when it was chosen (0 for players)
    pub target_zcc: Vec<Vec<u32>>,
    /// for an ability: its index in the source's abilities, and the source's zone-change count when it triggered
    pub ability: Option<(usize, u32)>,
    /// a spell cast with its kicker paid
    pub kicked: bool,
    /// the source's power as it last was on the battlefield, for an ability whose cost sacrificed it
    pub lki_power: Option<i32>,
    /// a delayed triggered ability (it has no ability index)
    pub delayed: Option<Arc<Delayed>>,
    /// a modal spell's chosen modes, in printed order (empty: the first)
    pub modes: Vec<usize>,
    /// the X announced as it was cast or activated
    pub x: i32,
}

/// An ability a resolved effect gave one object.
#[derive(Clone, Debug, PartialEq)]
pub struct Grant {
    pub target: ObjId,
    pub zcc: u32,
    pub ability: AbilityDef,
    pub until_end_of_turn: bool,
}

/// A delayed triggered ability a resolved effect set up: "sacrifice that token at end of combat".
#[derive(Clone, Debug, PartialEq)]
pub struct Delayed {
    pub rule: String,
    /// the permanents it sacrifices, with their zone-change counts
    pub sacrifice: Vec<(ObjId, u32)>,
}

impl StackItem {
    pub fn spell(card: ObjId, controller: Seat) -> StackItem {
        StackItem {
            card,
            controller,
            targets: Vec::new(),
            target_zcc: Vec::new(),
            ability: None,
            kicked: false,
            lki_power: None,
            delayed: None,
            modes: Vec::new(),
            x: 0,
        }
    }
}

/// A continuous effect from a resolved spell or ability: +power/+toughness for one object.
#[derive(Clone, Debug, PartialEq)]
pub struct Boost {
    pub target: ObjId,
    pub zcc: u32,
    pub power: i32,
    pub toughness: i32,
    /// ends in the cleanup step (otherwise, when the object leaves the battlefield)
    pub until_end_of_turn: bool,
}

/// A game: everything the rules look at. Cloning one is cheap (see the module docs).
#[derive(Clone, Debug)]
pub struct Game {
    pub rng: JavaRandom,
    pub turn: u32,
    pub step: Option<Step>,
    pub active: Option<Seat>,
    pub priority: Option<Seat>,
    pub players: Arc<Vec<Player>>,
    pub objs: Arc<Vec<Arc<Obj>>>,
    pub battlefield: Arc<Vec<ObjId>>,
    pub exile: Arc<Vec<ObjId>>,
    /// emblems, in the order they were made
    pub command: Arc<Vec<ObjId>>,
    /// bottom first: the top is the last
    pub stack: Arc<Vec<StackItem>>,
    pub boosts: Arc<Vec<Boost>>,
    /// triggered abilities waiting to be put on the stack, in the order they triggered
    pub pending: Vec<StackItem>,
    /// delayed triggers waiting for the end of combat
    pub at_end_of_combat: Vec<StackItem>,
    pub grants: Arc<Vec<Grant>>,
    /// permanents exiled instead if they would die this turn
    pub exile_if_dies: Vec<(ObjId, u32)>,
    /// cards exiled until a permanent (and its zone change count) leaves the battlefield
    pub exiled_until_leaves: Vec<(ObjId, u32, ObjId)>,
    /// cards to return now: their exiling permanent left (`OnLeaveReturnExiledAbility`, which uses no stack)
    pub returning: Vec<ObjId>,
    /// this combat has a first-strike damage step
    pub first_strike_step: bool,
    /// the game is over (a player lost, or it reached its last turn)
    pub over: bool,
}

impl Game {
    pub fn new(seats: usize, seed_state: i64) -> Game {
        let mut rng = JavaRandom::new(0);
        rng.set_state(seed_state);
        Game {
            rng,
            turn: 0,
            step: None,
            active: None,
            priority: None,
            players: Arc::new((0..seats).map(|_| Player { life: 20, ..Player::default() }).collect()),
            objs: Arc::new(Vec::new()),
            battlefield: Arc::new(Vec::new()),
            command: Arc::new(Vec::new()),
            exile: Arc::new(Vec::new()),
            stack: Arc::new(Vec::new()),
            boosts: Arc::new(Vec::new()),
            pending: Vec::new(),
            at_end_of_combat: Vec::new(),
            grants: Arc::new(Vec::new()),
            exile_if_dies: Vec::new(),
            exiled_until_leaves: Vec::new(),
            returning: Vec::new(),
            first_strike_step: false,
            over: false,
        }
    }

    pub fn seats(&self) -> usize {
        self.players.len()
    }

    pub fn obj(&self, id: ObjId) -> &Obj {
        &self.objs[id.0 as usize]
    }

    pub fn obj_mut(&mut self, id: ObjId) -> &mut Obj {
        Arc::make_mut(&mut Arc::make_mut(&mut self.objs)[id.0 as usize])
    }

    pub fn player(&self, seat: Seat) -> &Player {
        &self.players[seat]
    }

    pub fn player_mut(&mut self, seat: Seat) -> &mut Player {
        &mut Arc::make_mut(&mut self.players)[seat]
    }

    pub fn opponent(&self, seat: Seat) -> Seat {
        (seat + 1) % self.seats()
    }

    /// Creates a card owned by `owner` in `zone`, without putting it in that zone's list.
    pub fn create_obj(&mut self, def: Arc<CardDef>, owner: Seat, zone: Zone) -> ObjId {
        let id = ObjId(self.objs.len() as u32);
        Arc::make_mut(&mut self.objs).push(Arc::new(Obj {
            id,
            zcc: 0,
            def,
            owner,
            controller: owner,
            zone,
            tapped: false,
            damage: 0,
            counters: Vec::new(),
            face_down: false,
            attacking: None,
            attacking_pw: None,
            loyalty_used: false,
            kicked: false,
            until_end_of_turn_def: None,
            attached_to: None,
            blocking: Vec::new(),
            blocked: false,
            blocked_by: Vec::new(),
            sick: false,
            deathtouched: false,
            token: false,
            trigger_order: id.0 as u64,
        }));
        id
    }

    /// Whether `id` passes `filter`, as seen from `source` (whose controller is "you"; for targets, the chooser is
    /// passed as `you`). Works for objects in any zone: a spell's or a card's characteristics are its card's.
    pub fn filter_matches(&self, filter: &Filter, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
        filter.0.iter().all(|p| self.pred_holds(p, you, source, id))
    }

    fn pred_holds(&self, p: &Pred, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
        let o = self.obj(id);
        match p {
            Pred::Type { t } => self.has_type(id, *t),
            Pred::Subtype { s } => self.has_subtype(id, s),
            Pred::Supertype { s } => o.def.supertypes.iter().any(|x| x == s),
            Pred::Color { c } => o.def.colors.contains(c),
            Pred::Controller { who } => (o.controller == you) == (who == "you"),
            Pred::Owner { who } => (o.owner == you) == (who == "you"),
            Pred::Another => Some(id) != source,
            Pred::Mv { cmp, n } => cmp.holds(o.def.cost.mana_value() as i32, *n),
            Pred::Power { cmp, n } => cmp.holds(self.power_toughness(id).0, *n),
            Pred::TotalPowerToughness { n } => {
                let (p, t) = self.power_toughness(id);
                p + t <= *n
            }
            Pred::Toughness { cmp, n } => cmp.holds(self.power_toughness(id).1, *n),
            Pred::Tapped { v } => o.tapped == *v,
            Pred::Attacking => o.attacking.is_some(),
            Pred::InCombat => o.attacking.is_some() || !o.blocking.is_empty(),
            Pred::Token { v } => o.token == *v,
            Pred::Counter { name } => o.counter(name) > 0,
            Pred::Ability { a } => self.has_ability(id, a),
            Pred::Name { n } => o.name() == n,
            Pred::PermanentCard => o.def.is_permanent_card(),
            Pred::Not { x } => !self.pred_holds(x, you, source, id),
            Pred::Or { xs } => xs.iter().any(|x| self.pred_holds(x, you, source, id)),
            Pred::And { xs } => xs.iter().all(|x| self.pred_holds(x, you, source, id)),
        }
    }

    /// Whether an object has a card type now.
    pub fn has_type(&self, id: ObjId, t: CardType) -> bool {
        self.obj(id).is(t)
    }

    /// Whether an object has a subtype now.
    pub fn has_subtype(&self, id: ObjId, s: &str) -> bool {
        let def = &self.obj(id).def;
        def.subtypes.iter().any(|x| x == s) || (def.all_creature_types && !crate::card::LAND_TYPES.contains(&s))
    }

    /// Whether a permanent has an ability: printed, given by a resolved effect, or given by a static ability.
    pub fn has_ability(&self, id: ObjId, ability: &AbilityDef) -> bool {
        let o = self.obj(id);
        if o.has(ability) {
            return true;
        }
        if o.zone != Zone::Battlefield {
            return false;
        }
        if self.grants.iter().any(|g| g.target == id && g.zcc == o.zcc && g.ability == *ability) {
            return true;
        }
        self.battlefield.iter().chain(self.command.iter()).any(|src| {
            let s = self.obj(*src);
            s.def.abilities.iter().any(|a| match a {
                AbilityDef::SimpleStaticAbility { effects, .. } => {
                    effects.iter().any(|e| self.static_gives(e, *src, s.controller, id, ability))
                }
                _ => false,
            })
        })
    }

    /// Whether a condition holds for `you`, the controller of `source`. `Kicked` holds only where the kicked spell is
    /// known (`rules::condition_with_kicker`).
    /// The triggered abilities of `sources` that trigger on an event (`fires`: for a source, its controller and its
    /// trigger) and whose intervening "if" holds, in Java's registration order.
    pub fn triggers(
        &self,
        sources: impl IntoIterator<Item = ObjId>,
        mut fires: impl FnMut(&Game, ObjId, Seat, &Trigger) -> bool,
    ) -> Vec<StackItem> {
        let mut fired = Vec::new();
        for src in sources {
            let o = self.obj(src);
            for (i, a) in o.def.abilities.iter().enumerate() {
                if let Some(t) = a.trigger() {
                    if fires(self, src, o.controller, t) && self.condition_holds(a.intervening_if(), o.controller, src)
                    {
                        fired.push(StackItem { ability: Some((i, o.zcc)), ..StackItem::spell(src, o.controller) });
                    }
                }
            }
        }
        fired.sort_by_key(|t| self.obj(t.card).trigger_order);
        fired
    }

    /// `who` ("you", "opponent", "any") of a trigger, for its controller `you` and the player `seat` it's about.
    /// The permanent an Aura is attached to, while it's still that object on the battlefield.
    pub fn attached(&self, aura: ObjId) -> Option<ObjId> {
        let (to, zcc) = self.obj(aura).attached_to?;
        let o = self.obj(to);
        (o.zone == Zone::Battlefield && o.zcc == zcc).then_some(to)
    }

    /// Whether a static ability's effect on the battlefield matches (for effects that apply to the whole game).
    pub fn any_static(&self, mut f: impl FnMut(&Game, ObjId, &EffectDef) -> bool) -> bool {
        self.battlefield.iter().any(|src| {
            self.obj(*src).def.abilities.iter().any(|a| match a {
                AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| f(self, *src, e)),
                _ => false,
            })
        })
    }

    /// Whether a static ability of an attached Aura says `id` can't attack or block.
    pub fn cant_attack_or_block(&self, id: ObjId) -> bool {
        self.battlefield.iter().any(|src| {
            self.attached(*src) == Some(id)
                && self.obj(*src).def.abilities.iter().any(|a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => {
                        effects.iter().any(|e| matches!(e, EffectDef::CantAttackBlockAttachedEffect))
                    }
                    _ => false,
                })
        })
    }

    /// What triggered abilities can come from: permanents, then emblems.
    pub fn trigger_sources(&self) -> Vec<ObjId> {
        self.battlefield.iter().chain(self.command.iter()).copied().collect()
    }

    pub fn whose(who: &str, you: Seat, seat: Seat) -> bool {
        match who {
            "you" => you == seat,
            "opponent" => you != seat,
            _ => true,
        }
    }

    /// An effect's number for `you`, the controller of `source`, with `x` announced.
    pub fn value(&self, a: &Amount, you: Seat, source: ObjId, x: i32) -> i32 {
        match a {
            Amount::Fixed(n) => *n,
            Amount::Of(v) => match v {
                Value::X => x,
                Value::Permanents { filter } => self
                    .battlefield
                    .iter()
                    .filter(|id| self.filter_matches(filter, you, Some(source), **id))
                    .count() as i32,
                Value::CardsInHand => self.player(you).hand.len() as i32,
                Value::CardsInGraveyard { filter } => self
                    .player(you)
                    .graveyard
                    .iter()
                    .filter(|id| self.filter_matches(filter, you, Some(source), **id))
                    .count() as i32,
                Value::SourcePower => self.power_toughness(source).0,
                Value::Negative { of } => -self.value(of, you, source, x),
                Value::Times { of, n } => self.value(of, you, source, x) * n,
                Value::Half { of } => self.value(of, you, source, x) / 2,
                Value::Plus { of, n } => self.value(of, you, source, x) + n,
                Value::CardTypesInGraveyards => {
                    let mut types: Vec<CardType> = Vec::new();
                    for p in self.players.iter() {
                        for c in p.graveyard.iter() {
                            for t in &self.obj(*c).def.types {
                                if !types.contains(t) {
                                    types.push(*t);
                                }
                            }
                        }
                    }
                    types.len() as i32
                }
                Value::Devotion { c } => self
                    .battlefield
                    .iter()
                    .filter(|b| self.obj(**b).controller == you)
                    .map(|b| self.obj(*b).def.cost.colored[c.index()] as i32)
                    .sum(),
            },
        }
    }

    pub fn condition_holds(&self, condition: Option<&Condition>, you: Seat, source: ObjId) -> bool {
        let Some(c) = condition else { return true };
        match c {
            // a permanent that was cast kicked ("when this enters, if it was kicked")
            Condition::Kicked => self.obj(source).kicked,
            Condition::Raid => self.player(you).attacked,
            Condition::Revolt => self.player(you).permanent_left,
            Condition::InstantsSorceriesCast { n } => self.player(you).instants_sorceries_cast >= *n,
            Condition::SourceTargets { filter } => self.stack.iter().any(|i| {
                i.card == source
                    && i.ability.is_none()
                    && i.targets.iter().flatten().any(|t| match t {
                        Target::Obj(o) => self.filter_matches(filter, you, Some(source), *o),
                        _ => false,
                    })
            }),
            Condition::SourceAttacking => self.obj(source).attacking.is_some(),
            Condition::CardsInGraveyard { filter, n } => {
                let g = &self.player(you).graveyard;
                g.iter().filter(|c| self.filter_matches(filter, you, Some(source), **c)).count() as i32 >= *n
            }
            Condition::OpponentsLostLife => (0..self.seats()).any(|p| p != you && self.player(p).life_lost > 0),
            Condition::MyTurn => self.active == Some(you),
            Condition::Permanents { filter, cmp, n, all } => {
                let count = self
                    .battlefield
                    .iter()
                    .filter(|b| {
                        (*all || self.obj(**b).controller == you) && self.filter_matches(filter, you, Some(source), **b)
                    })
                    .count() as i32;
                cmp.holds(count, *n)
            }
            Condition::SourceCounter { name, n } => {
                let o = self.obj(source);
                o.zone == Zone::Battlefield && o.counter(name) >= *n
            }
            Condition::Not { x } => !self.condition_holds(Some(x), you, source),
        }
    }

    /// Whether a static ability's effect, from `src`, gives `id` the ability.
    fn static_gives(&self, e: &EffectDef, src: ObjId, you: Seat, id: ObjId, ability: &AbilityDef) -> bool {
        match e {
            EffectDef::GainAbilityAllEffect { ability: given, filter, exclude_source, .. } => {
                **given == *ability
                    && !(*exclude_source && id == src)
                    && self.filter_matches(filter, you, Some(src), id)
            }
            EffectDef::GainAbilityControlledEffect { ability: given, filter, exclude_source, .. } => {
                **given == *ability
                    && !(*exclude_source && id == src)
                    && self.obj(id).controller == you
                    && self.filter_matches(filter, you, Some(src), id)
            }
            EffectDef::GainAbilitySourceEffect { ability: given, .. } => **given == *ability && id == src,
            EffectDef::ConditionalContinuousEffect { effect, condition } => {
                self.condition_holds(Some(condition), you, src) && self.static_gives(effect, src, you, id, ability)
            }
            _ => false,
        }
    }

    /// Power and toughness with continuous effects applied: static abilities of permanents, then resolved boosts
    /// (layer 7c; everything here only adds, so the order doesn't matter yet).
    pub fn power_toughness(&self, id: ObjId) -> (i32, i32) {
        let o = self.obj(id);
        let (mut p, mut t) = (o.def.power, o.def.toughness);
        if o.zone != Zone::Battlefield {
            return (p, t);
        }
        // layer 7a: characteristic-defining abilities
        for a in &o.def.abilities {
            if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                for e in effects {
                    if let EffectDef::SetBasePowerToughnessPlusOneSourceEffect { amount } = e {
                        p = self.value(amount, o.controller, id, 0);
                        t = p + 1;
                    }
                    if let EffectDef::SetBasePowerToughnessSourceEffect { count } = e {
                        if count == "creatures_you_control" {
                            let n = self
                                .battlefield
                                .iter()
                                .filter(|b| {
                                    let x = self.obj(**b);
                                    x.controller == o.controller && x.is(CardType::Creature)
                                })
                                .count() as i32;
                            p = n;
                            t = n;
                        }
                    }
                }
            }
        }
        let counters = o.counter("+1/+1");
        p += counters;
        t += counters;
        for src in self.battlefield.iter().chain(self.command.iter()) {
            let s = self.obj(*src);
            for a in &s.def.abilities {
                if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                    for e in effects {
                        let (dp, dt) = self.static_boost(e, *src, s.controller, id);
                        p += dp;
                        t += dt;
                    }
                }
            }
        }
        for b in self.boosts.iter() {
            if b.target == id && b.zcc == o.zcc {
                p += b.power;
                t += b.toughness;
            }
        }
        (p, t)
    }

    /// What a static ability's effect, from `src` controlled by `you`, adds to `id`'s power and toughness.
    fn static_boost(&self, e: &EffectDef, src: ObjId, you: Seat, id: ObjId) -> (i32, i32) {
        match e {
            EffectDef::BoostControlledEffect { power, toughness, filter, exclude_source, .. }
                if self.obj(id).controller == you
                    && !(*exclude_source && src == id)
                    && self.filter_matches(filter, you, Some(src), id) =>
            {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
            }
            EffectDef::BoostAllEffect { power, toughness, filter, exclude_source, .. }
                if !(*exclude_source && src == id) && self.filter_matches(filter, you, Some(src), id) =>
            {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
            }
            EffectDef::ConditionalContinuousEffect { effect, condition }
                if self.condition_holds(Some(condition), you, src) =>
            {
                self.static_boost(effect, src, you, id)
            }
            EffectDef::BoostEnchantedEffect { power, toughness } if self.attached(src) == Some(id) => {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
            }
            _ => (0, 0),
        }
    }

    /// A stack object's name as Java gives it: the card's, or `stack ability (<rule text>)`.
    pub fn stack_name(&self, item: &StackItem) -> String {
        let card = self.obj(item.card);
        if let Some(d) = &item.delayed {
            return format!("stack ability ({})", d.rule);
        }
        match item.ability {
            Some((i, _)) => {
                let rule = card.def.abilities.get(i).and_then(|a| a.on_stack()).map(|(r, ..)| r).unwrap_or("");
                format!("stack ability ({})", rule)
            }
            None => card.name().to_string(),
        }
    }

    /// Takes an object out of whatever zone list holds it.
    pub fn remove_from_zone(&mut self, id: ObjId) {
        let (zone, owner) = {
            let o = self.obj(id);
            (o.zone, o.owner)
        };
        match zone {
            Zone::Library => self.player_mut(owner).library.retain(|x| *x != id),
            Zone::Hand => self.player_mut(owner).hand.retain(|x| *x != id),
            Zone::Graveyard => self.player_mut(owner).graveyard.retain(|x| *x != id),
            Zone::Battlefield => Arc::make_mut(&mut self.battlefield).retain(|x| *x != id),
            Zone::Exile => Arc::make_mut(&mut self.exile).retain(|x| *x != id),
            Zone::Command => Arc::make_mut(&mut self.command).retain(|x| *x != id),
            Zone::Stack => Arc::make_mut(&mut self.stack).retain(|x| x.card != id || x.ability.is_some()),
        }
    }

    /// Moves an object to a zone (its owner's, for hands, libraries and graveyards), at the end of that zone's list
    /// (a library's bottom). A permanent leaving the battlefield forgets its state.
    ///
    /// A creature going from the battlefield to a graveyard dies: its "when this dies" triggers wait. A token that
    /// leaves the battlefield is gone (it goes to no zone's list).
    pub fn move_to(&mut self, id: ObjId, zone: Zone) {
        let from = self.obj(id).zone;
        // "if it would die this turn, exile it instead"
        let zone = if from == Zone::Battlefield
            && zone == Zone::Graveyard
            && self.exile_if_dies.contains(&(id, self.obj(id).zcc))
        {
            Zone::Exile
        } else {
            zone
        };
        // dies triggers look back at the battlefield as it was (this one included)
        let dies = if from == Zone::Battlefield && zone == Zone::Graveyard {
            let sources: Vec<ObjId> = self.battlefield.iter().copied().collect();
            self.triggers(sources, |g, src, you, t| match t {
                Trigger::Dies | Trigger::EntersOrDies | Trigger::LeavesBattlefield => src == id,
                // the source itself always counts ("this or another"); others must match
                Trigger::DiesAll { filter, include_self } => {
                    if src == id {
                        *include_self
                    } else {
                        g.filter_matches(filter, you, Some(src), id)
                    }
                }
                _ => false,
            })
        } else if from == Zone::Battlefield && zone != Zone::Battlefield {
            self.triggers([id], |_, src, _, t| matches!(t, Trigger::LeavesBattlefield) && src == id)
        } else {
            Vec::new()
        };
        if from == Zone::Battlefield && zone != Zone::Battlefield {
            let zcc = self.obj(id).zcc;
            let (back, kept): (Vec<_>, Vec<_>) =
                self.exiled_until_leaves.iter().copied().partition(|(s, z, _)| *s == id && *z == zcc);
            self.exiled_until_leaves = kept;
            self.returning.extend(back.into_iter().map(|(_, _, c)| c));
        }
        if from == Zone::Battlefield && zone != Zone::Battlefield {
            let c = self.obj(id).controller;
            self.player_mut(c).permanent_left = true;
        }
        self.remove_from_zone(id);
        let owner = self.obj(id).owner;
        {
            let o = self.obj_mut(id);
            if zone != Zone::Battlefield {
                o.tapped = false;
                o.damage = 0;
                o.counters.clear();
                o.attacking = None;
                o.attacking_pw = None;
                o.loyalty_used = false;
                o.kicked = false;
                o.attached_to = None;
                if let Some(def) = o.until_end_of_turn_def.take() {
                    o.def = def;
                }
                o.blocking.clear();
                o.blocked = false;
                o.blocked_by.clear();
                o.controller = o.owner;
                o.deathtouched = false;
            }
            o.zone = zone;
            o.zcc += 1;
        }
        for mut t in dies {
            // a dead permanent's own trigger is its graveyard card's
            if t.card == id {
                t.ability = t.ability.map(|(i, _)| (i, self.obj(id).zcc));
            }
            self.pending.push(t);
        }
        if self.obj(id).token && zone != Zone::Battlefield {
            return;
        }
        match zone {
            Zone::Library => self.player_mut(owner).library.push(id),
            Zone::Hand => self.player_mut(owner).hand.push(id),
            Zone::Graveyard => self.player_mut(owner).graveyard.push(id),
            Zone::Battlefield => {
                self.obj_mut(id).sick = true;
                Arc::make_mut(&mut self.battlefield).push(id)
            }
            Zone::Exile => Arc::make_mut(&mut self.exile).push(id),
            Zone::Command => Arc::make_mut(&mut self.command).push(id),
            Zone::Stack => {}
        }
    }
}
