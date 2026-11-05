//! The game state. Each part sits behind an `Arc` and is copied only when changed (`Arc::make_mut`), so copying a
//! `Game` for an AI search node bumps a few reference counts.

use crate::card::{AbilityDef, CardDef, CardType, EffectDef, Filter, Mana};
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
    pub targets: Vec<Target>,
    /// each object target's zone-change count when it was chosen (0 for players)
    pub target_zcc: Vec<u32>,
    /// for an ability: its index in the source's abilities, and the source's zone-change count when it triggered
    pub ability: Option<(usize, u32)>,
    /// a spell cast with its kicker paid
    pub kicked: bool,
    /// the source's power as it last was on the battlefield, for an ability whose cost sacrificed it
    pub lki_power: Option<i32>,
    /// a delayed triggered ability (it has no ability index)
    pub delayed: Option<Arc<Delayed>>,
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
    /// bottom first: the top is the last
    pub stack: Arc<Vec<StackItem>>,
    pub boosts: Arc<Vec<Boost>>,
    /// triggered abilities waiting to be put on the stack, in the order they triggered
    pub pending: Vec<StackItem>,
    /// delayed triggers waiting for the end of combat
    pub at_end_of_combat: Vec<StackItem>,
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
            exile: Arc::new(Vec::new()),
            stack: Arc::new(Vec::new()),
            boosts: Arc::new(Vec::new()),
            pending: Vec::new(),
            at_end_of_combat: Vec::new(),
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

    /// Whether `id` is one of `filter`'s permanents, as seen from `source` (whose controller is "you"; for targets,
    /// the chooser is passed as `you`).
    pub fn filter_matches(&self, filter: Filter, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
        let o = self.obj(id);
        match filter {
            Filter::Creatures => o.is(CardType::Creature),
            Filter::ControlledCreaturesWithP1p1 => {
                o.is(CardType::Creature) && o.controller == you && o.counter("+1/+1") > 0
            }
            Filter::AnotherCreatureYouControl => o.is(CardType::Creature) && o.controller == you && Some(id) != source,
            Filter::CreaturesOrPlaneswalkers => o.is(CardType::Creature) || o.is(CardType::Planeswalker),
            Filter::OpponentsCreatureOrPlaneswalker => {
                (o.is(CardType::Creature) || o.is(CardType::Planeswalker)) && o.controller != you
            }
        }
    }

    /// Whether a permanent has an ability, printed or given by a static ability (`GainAbilityAllEffect`).
    pub fn has_ability(&self, id: ObjId, ability: &AbilityDef) -> bool {
        let o = self.obj(id);
        if o.has(ability) {
            return true;
        }
        if o.zone != Zone::Battlefield {
            return false;
        }
        self.battlefield.iter().any(|src| {
            let s = self.obj(*src);
            s.def.abilities.iter().any(|a| match a {
                AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| match e {
                    EffectDef::GainAbilityAllEffect { ability: given, filter, .. } => {
                        **given == *ability && self.filter_matches(*filter, s.controller, Some(*src), id)
                    }
                    _ => false,
                }),
                _ => false,
            })
        })
    }

    /// Power and toughness with continuous effects applied: static abilities of permanents, then resolved boosts
    /// (layer 7c; everything here only adds, so the order doesn't matter yet).
    pub fn power_toughness(&self, id: ObjId) -> (i32, i32) {
        let o = self.obj(id);
        let (mut p, mut t) = (o.def.power, o.def.toughness);
        if o.zone != Zone::Battlefield {
            return (p, t);
        }
        let counters = o.counter("+1/+1");
        p += counters;
        t += counters;
        for src in self.battlefield.iter() {
            let s = self.obj(*src);
            for a in &s.def.abilities {
                if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                    for e in effects {
                        if let EffectDef::BoostControlledEffect { power, toughness, filter, exclude_source, .. } = e {
                            let applies = self.filter_matches(*filter, s.controller, Some(*src), id);
                            if applies && o.controller == s.controller && !(*exclude_source && *src == id) {
                                p += power;
                                t += toughness;
                            }
                        }
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
            Zone::Stack => Arc::make_mut(&mut self.stack).retain(|x| x.card != id || x.ability.is_some()),
        }
    }

    /// Moves an object to a zone (its owner's, for hands, libraries and graveyards), at the end of that zone's list
    /// (a library's bottom). A permanent leaving the battlefield forgets its state.
    ///
    /// A creature going from the battlefield to a graveyard dies: its "when this dies" triggers wait. A token that
    /// leaves the battlefield is gone (it goes to no zone's list).
    pub fn move_to(&mut self, id: ObjId, zone: Zone) {
        let (from, controller) = (self.obj(id).zone, self.obj(id).controller);
        self.remove_from_zone(id);
        let owner = self.obj(id).owner;
        {
            let o = self.obj_mut(id);
            if zone != Zone::Battlefield {
                o.tapped = false;
                o.damage = 0;
                o.counters.clear();
                o.attacking = None;
                o.blocking.clear();
                o.blocked = false;
                o.blocked_by.clear();
                o.controller = o.owner;
                o.deathtouched = false;
            }
            o.zone = zone;
            o.zcc += 1;
        }
        if from == Zone::Battlefield && zone == Zone::Graveyard {
            let (def, zcc) = (self.obj(id).def.clone(), self.obj(id).zcc);
            for (i, a) in def.abilities.iter().enumerate() {
                if matches!(a, AbilityDef::DiesSourceTriggeredAbility { .. }) {
                    self.pending.push(StackItem { ability: Some((i, zcc)), ..StackItem::spell(id, controller) });
                }
            }
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
            Zone::Stack => {}
        }
    }
}
