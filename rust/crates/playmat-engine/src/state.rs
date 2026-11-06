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
    /// it entered the battlefield by resolving as a spell
    pub cast: bool,
    /// what it was before an effect changed its characteristics (`def`) until end of turn
    pub until_end_of_turn_def: Option<Arc<CardDef>>,
    /// the X it was cast with, while it's on the battlefield
    pub cast_x: i32,
    /// what it was before an effect changed its characteristics (`def`) for as long as it stays on the battlefield
    pub printed_def: Option<Arc<CardDef>>,
    /// the permanent (and its zone change count) this Aura is attached to
    pub attached_to: Option<(ObjId, u32)>,
    /// cast as its adventure: the whole card, while `def` is the adventure's
    pub adventurer: Option<Arc<CardDef>>,
    /// a modal double-faced card played as its back face: the whole card, back when it leaves the battlefield
    pub front: Option<Arc<CardDef>>,
    /// a spell cast with flashback: exiled instead of going anywhere else from the stack
    pub flashback: bool,
    /// cast with its escape cost (until it enters, or leaves the stack another way)
    pub escaped: bool,
    /// the player the attacker this ninja's ninjutsu returned was attacking
    pub ninjutsu_defender: Option<Seat>,
    /// the permanent chosen as it entered (`ChooseCreatureEffect`), kept after it leaves
    pub chosen: Option<(ObjId, u32)>,
    /// exerted: doesn't untap during its controller's next untap step
    pub exerted: bool,
    /// its power-up ability was activated (once each game, for this object)
    pub powered_up: bool,
    /// creature types and base power and toughness it has until it leaves the battlefield (Evolved Sleeper: each
    /// "becomes" replaces its creature types)
    pub creature_types: Option<Vec<String>>,
    pub base_pt: Option<(i32, i32)>,
    /// regeneration shields until end of turn
    pub regen_shields: u32,
    /// "if this creature would die, exile it instead" for as long as it stays (Can't Stay Away)
    pub exile_instead_of_dying: bool,
    /// `DamagedByControlledWatcher`: the turn, and the players whose sources dealt it damage that turn (bits)
    pub damaged_by: (u32, u8),
    /// it lost all abilities while this source (and zone-change count) is on the battlefield: the source, and its
    /// card as it was
    pub lost_abilities: Option<(ObjId, u32, Arc<CardDef>)>,
    /// in exile: this player may play it through the end of this turn number
    pub may_play: Option<(Seat, u32)>,
    /// cast from exile, it costs this much more (Elite Spellbinder)
    pub play_tax: u32,
    /// cast for its warp cost (on the stack, then on the battlefield)
    pub warped: bool,
    /// cast for its blitz cost (until it leaves the battlefield)
    pub blitzed: bool,
    /// cast with its offspring cost paid
    pub offspring: bool,
    /// in exile after warp: its owner may cast it from this turn number on
    pub warp_from: Option<u32>,
    /// the mana spent to cast it, as a spell (`ManaSpentToCastCount`)
    pub mana_spent: u32,
    /// the turn it last entered the battlefield
    pub entered_turn: u32,
    /// the turn and how many times this permanent became a target in it
    pub targeted: (u32, u32),
    /// exiled "on an adventure": its owner may cast the creature from exile
    pub on_adventure: bool,
    /// the zone Java keeps for an adventurer card's adventure part (`CardWithSpellOption`): it follows the card,
    /// except when the card goes onto the stack or the battlefield or leaves the battlefield
    pub adventure_zone: Zone,
    /// the attackers it blocks
    pub blocking: Vec<ObjId>,
    /// how many attackers it was declared to block (Java's `blocking` count: an attacker leaving combat doesn't
    /// lower it)
    pub blocks: u32,
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
    /// how much of the pool's {R} lasts until end of combat (firebending)
    pub combat_red: u32,
    pub passed: bool,
    /// drew from an empty library since the last state-based check
    pub drew_from_empty: bool,
    /// can't gain life for the rest of the game (Screaming Nemesis)
    pub cant_gain_life: bool,
    /// declared an attacker this turn (`PlayerAttackedWatcher`)
    pub attacked: bool,
    /// life lost this turn
    pub life_lost: i32,
    /// cards drawn this turn
    pub drawn: u32,
    /// noncreature spells cast this turn
    pub noncreature_cast: u32,
    /// cast a non-Lemur creature spell with flying this turn
    pub flier_cast: bool,
    /// times they gained life this turn
    pub life_gains: u32,
    /// life they gained this turn
    pub life_gained: i32,
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
    /// an ability's face, when its source has transformed since it triggered or was activated
    pub def: Option<Arc<CardDef>>,
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
    /// a ward trigger: the ability it counters (its source and its `ability`); a spell is its target instead
    pub ward_of: Option<(ObjId, (usize, u32))>,
    /// an evolve trigger: the creature that entered
    pub entering: Option<ObjId>,
    /// gift: promised as it was cast, and the opponent it was promised to
    pub gift_promised: bool,
    pub gift_to: Option<Seat>,
}

/// An ability a resolved effect gave one object.
#[derive(Clone, Debug, PartialEq)]
pub struct Grant {
    pub target: ObjId,
    pub zcc: u32,
    pub ability: AbilityDef,
    pub until_end_of_turn: bool,
    /// it lasts until this player's next turn
    pub until_turn_of: Option<Seat>,
}

/// A delayed triggered ability a resolved effect set up: "sacrifice that token at end of combat".
#[derive(Clone, Debug, PartialEq)]
pub struct Delayed {
    pub rule: String,
    /// the permanents it sacrifices, with their zone-change counts
    pub sacrifice: Vec<(ObjId, u32)>,
    /// warp: the permanent it exiles (castable by its owner from exile on a later turn)
    pub warp_exile: Vec<(ObjId, u32)>,
    /// a reflexive trigger's effects, from its source as it was (`source_zcc`)
    pub effects: Vec<crate::card::EffectDef>,
    pub source_zcc: u32,
    /// the permanents its effects act on, while they're still those objects (`FixedTarget`)
    pub fixed: Vec<(ObjId, u32)>,
    /// earthbend: the card it returns to the battlefield tapped, while it's still that card in a graveyard or exile
    pub return_tapped: Vec<(ObjId, u32)>,
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
            ward_of: None,
            entering: None,
            gift_promised: false,
            gift_to: None,
            def: None,
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
    /// day (true) or night (false), once a daybound or nightbound permanent appeared
    pub day: Option<bool>,
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
    /// "at the beginning of the next end step" delayed triggers
    pub at_next_end_step: Vec<StackItem>,
    /// "when you next cast a creature spell this turn" delayed triggers: whose, the turn, and the trigger
    pub next_creature_spell: Vec<(Seat, u32, StackItem)>,
    /// spells that enter with more +1/+1 counters: the card as it was on the stack, and how many
    pub enter_counters: Vec<(ObjId, u32, i32)>,
    /// decision logs name exiled cards in the order the cards were created (`DecisionLog.setStartOrder`), not the
    /// order they were exiled
    pub exile_by_creation: bool,
    /// a creature died this turn (`MorbidWatcher`)
    pub creature_died: bool,
    /// questions Java asked where the engine has no one to ask (`move_to`): which of several replacement effects
    /// applies, when they all do the same; asked before the next question or trace
    pub owed_choices: Vec<Seat>,
    /// extra turns to take, the last added first (`TurnMod`)
    pub extra_turns: Vec<Seat>,
    pub grants: Arc<Vec<Grant>>,
    /// permanents exiled instead if they would die this turn
    pub exile_if_dies: Vec<(ObjId, u32)>,
    /// triggered abilities with a limit each turn that triggered this turn: source, ability index
    pub triggered_this_turn: Vec<(ObjId, usize)>,
    /// activated abilities with a limit each turn activated this turn: source, ability index
    pub activated_this_turn: Vec<(ObjId, usize)>,
    /// the battlefield as the current action began, for the dies triggers of what it moves all at once
    pub lookback: Vec<ObjId>,
    /// cards exiled until a permanent (and its zone change count) leaves the battlefield
    pub exiled_until_leaves: Vec<(ObjId, u32, ObjId)>,
    /// earthbent lands (with their zone change counts): when one dies or is exiled, it returns tapped
    pub earthbent: Vec<(ObjId, u32)>,
    /// cards exiled into a permanent's own exile zone (its id and zone change count), that stay there
    pub exiled_for_source: Vec<(ObjId, u32, ObjId)>,
    /// cards to return now: their exiling permanent left (`OnLeaveReturnExiledAbility`, which uses no stack), with
    /// the seat whose trigger that is
    pub returning: Vec<(Seat, ObjId)>,
    /// damage permanents were dealt since triggers were last checked, in order (`DAMAGED_BATCH_FOR_ONE_PERMANENT`)
    pub damaged_batch: Vec<(ObjId, i32)>,
    /// cards exiled until something leaves that go back to their owner's hand, not the battlefield (Kitesail
    /// Freebooter's)
    pub return_to_hand: Vec<ObjId>,
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
            at_next_end_step: Vec::new(),
            next_creature_spell: Vec::new(),
            enter_counters: Vec::new(),
            exile_by_creation: false,
            creature_died: false,
            owed_choices: Vec::new(),
            extra_turns: Vec::new(),
            grants: Arc::new(Vec::new()),
            exile_if_dies: Vec::new(),
            triggered_this_turn: Vec::new(),
            activated_this_turn: Vec::new(),
            lookback: Vec::new(),
            exiled_until_leaves: Vec::new(),
            earthbent: Vec::new(),
            exiled_for_source: Vec::new(),
            returning: Vec::new(),
            day: None,
            damaged_batch: Vec::new(),
            return_to_hand: Vec::new(),
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
            adventure_zone: zone,
            tapped: false,
            damage: 0,
            counters: Vec::new(),
            face_down: false,
            attacking: None,
            attacking_pw: None,
            loyalty_used: false,
            kicked: false,
            cast: false,
            until_end_of_turn_def: None,
            printed_def: None,
            cast_x: 0,
            attached_to: None,
            adventurer: None,
            front: None,
            flashback: false,
            escaped: false,
            ninjutsu_defender: None,
            chosen: None,
            exerted: false,
            powered_up: false,
            creature_types: None,
            base_pt: None,
            regen_shields: 0,
            exile_instead_of_dying: false,
            damaged_by: (0, 0),
            lost_abilities: None,
            may_play: None,
            play_tax: 0,
            warped: false,
            blitzed: false,
            offspring: false,
            warp_from: None,
            mana_spent: 0,
            entered_turn: 0,
            targeted: (0, 0),
            on_adventure: false,
            blocking: Vec::new(),
            blocks: 0,
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
            Pred::Adventure => self.obj(id).def.adventure.is_some() || self.obj(id).adventurer.is_some(),
            Pred::Mv { cmp, n } => cmp.holds(o.def.mana_value() as i32, *n),
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
        let o = self.obj(id);
        let def = &o.def;
        if let Some(types) = o.creature_types.as_ref().filter(|_| o.zone == Zone::Battlefield) {
            return types.iter().any(|x| x == s) || (crate::card::LAND_TYPES.contains(&s) && def.subtypes.iter().any(|x| x == s));
        }
        def.subtypes.iter().any(|x| x == s) || (def.all_creature_types && !crate::card::LAND_TYPES.contains(&s))
    }

    /// Whether a permanent has an ability: printed, given by a resolved effect, or given by a static ability.
    pub fn has_ability(&self, id: ObjId, ability: &AbilityDef) -> bool {
        let o = self.obj(id);
        if o.has(ability) {
            return true;
        }
        // an indestructible counter
        if *ability == AbilityDef::IndestructibleAbility && o.counter("indestructible") > 0 {
            return true;
        }
        if *ability == AbilityDef::DeathtouchAbility && o.counter("deathtouch") > 0 {
            return true;
        }
        if *ability == AbilityDef::FlyingAbility && o.counter("flying") > 0 {
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
                        && self.condition_holds(a.trigger_condition(), o.controller, src)
                    {
                        fired.push(StackItem { ability: Some((i, o.zcc)), ..StackItem::spell(src, o.controller) });
                    }
                }
            }
        }
        fired.sort_by_key(|t| self.obj(t.card).trigger_order);
        fired
    }

    /// Pelt Collector's triggers on `id` entering or dying: each remembers `id`'s power, and fires only if it's
    /// greater than its source's
    pub fn pelt_collector(&self, mut fired: Vec<StackItem>, id: ObjId) -> Vec<StackItem> {
        let power = self.power_toughness(id).0;
        for t in fired.iter_mut() {
            if self.trigger_of(t) == Some(&Trigger::PeltCollector) {
                t.lki_power = Some(power);
            }
        }
        fired.retain(|t| self.pelt_holds(t));
        fired
    }

    pub fn trigger_of<'a>(&'a self, t: &'a StackItem) -> Option<&'a Trigger> {
        t.ability.and_then(|(i, _)| self.item_def(t).abilities.get(i)).and_then(|a| a.trigger())
    }

    /// The card face a spell or ability comes from (an ability keeps the face it came from)
    pub fn item_def<'a>(&'a self, t: &'a StackItem) -> &'a Arc<CardDef> {
        t.def.as_ref().unwrap_or(&self.obj(t.card).def)
    }

    /// Pelt Collector's intervening "if": the creature's power is greater than its source's (still on the
    /// battlefield)
    pub fn pelt_holds(&self, t: &StackItem) -> bool {
        if self.trigger_of(t) != Some(&Trigger::PeltCollector) {
            return true;
        }
        let o = self.obj(t.card);
        let here = o.zone == Zone::Battlefield && t.ability.is_some_and(|(_, zcc)| zcc == o.zcc);
        here && t.lki_power.unwrap_or(0) > self.power_toughness(t.card).0
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
    /// Lovestruck Beast's restriction: it can't attack while its controller has no 1/1 creature
    pub fn cant_attack_alone(&self, id: ObjId) -> bool {
        let you = self.obj(id).controller;
        self.any_static(|g, src, e| {
            src == id
                && matches!(e, EffectDef::LovestruckBeastEffect)
                && !g.battlefield.iter().any(|c| {
                    let o = g.obj(*c);
                    o.controller == you && o.is(CardType::Creature) && g.power_toughness(*c) == (1, 1)
                })
        })
    }

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
                Value::CardsDrawn => self.player(you).drawn as i32,
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
                Value::GreatestManaValue { filter } => self
                    .battlefield
                    .iter()
                    .filter(|b| self.filter_matches(filter, you, Some(source), **b))
                    .map(|b| self.obj(*b).def.mana_value() as i32)
                    .max()
                    .unwrap_or(0),
                Value::SourceCounters { counter } => {
                    if self.obj(source).zone == Zone::Battlefield { self.obj(source).counter(counter) } else { 0 }
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
            Condition::Offspring => self.obj(source).offspring,
            Condition::CastFromEverywhere => self.obj(source).cast,
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
            Condition::OpponentControlsMore { filter } => {
                let count = |p: Seat| {
                    self.battlefield
                        .iter()
                        .filter(|b| self.obj(**b).controller == p && self.filter_matches(filter, you, Some(source), **b))
                        .count()
                };
                (0..self.seats()).any(|p| p != you && count(p) > count(you))
            }
            Condition::OpponentMoreLife => {
                (0..self.seats()).any(|p| p != you && self.player(p).life > self.player(you).life)
            }
            Condition::OpponentMoreCards => {
                (0..self.seats()).any(|p| p != you && self.player(p).hand.len() > self.player(you).hand.len())
            }
            Condition::OpponentsLostLife => (0..self.seats()).any(|p| p != you && self.player(p).life_lost > 0),
            Condition::MyTurn => self.active == Some(you),
            Condition::Morbid => self.creature_died,
            Condition::FlierCast => self.player(you).flier_cast,
            Condition::YouGainedLife => self.player(you).life_gained > 0,
            Condition::DrewCards { n } => self.player(you).drawn >= *n,
            Condition::ExiledCardTypes { n } => {
                let zcc = self.obj(source).zcc;
                let mut types: Vec<CardType> = Vec::new();
                for (_, _, c) in self.exiled_for_source.iter().filter(|(s, z, c)| *s == source && *z == zcc && self.obj(*c).zone == Zone::Exile) {
                    for t in &self.obj(*c).def.types {
                        if !types.contains(t) {
                            types.push(*t);
                        }
                    }
                }
                types.len() >= *n
            }
            Condition::SourceEnteredThisTurn => self.obj(source).entered_turn == self.turn,
            Condition::StartingTown => self.active == Some(you) && self.turn.div_ceil(self.seats() as u32) <= 3,
            Condition::LifeCompare { who, cmp, n } => (0..self.seats())
                .filter(|s| if who == "you" { *s == you } else { *s != you })
                .any(|s| cmp.holds(self.player(s).life, *n)),
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
            Condition::And { xs } => xs.iter().all(|x| self.condition_holds(Some(x), you, source)),
            Condition::Or { xs } => xs.iter().any(|x| self.condition_holds(Some(x), you, source)),
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
            EffectDef::GainAbilityAttachedEffect { ability: given } => **given == *ability && self.attached(src) == Some(id),
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
                    if let EffectDef::SetBasePowerSourceEffect { amount } = e {
                        p = self.value(amount, o.controller, id, 0);
                    }
                    if let EffectDef::SetBasePowerToughnessPlusOneSourceEffect { amount } = e {
                        p = self.value(amount, o.controller, id, 0);
                        t = p + 1;
                    }
                    if let EffectDef::SetBasePowerToughnessSourceEffect { count } = e {
                        if count == "exiled_with_source" {
                            let n = self.exiled_for_source.iter().filter(|(s, z, _)| *s == id && *z == o.zcc).count() as i32;
                            p = n;
                            t = n;
                        }
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
        // layer 7b
        if let Some((bp, bt)) = o.base_pt {
            (p, t) = (bp, bt);
        }
        // +1/+1 counters change only a creature's power and toughness (a Vehicle's, while it's crewed)
        if self.has_type(id, CardType::Creature) {
            let counters = o.counter("+1/+1") - o.counter("-1/-1");
            p += counters;
            t += counters;
        }
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
            // a static "this gets +N/+N" (inside a conditional one)
            EffectDef::BoostSourceEffect { power, toughness, .. } if src == id => {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
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
                let rule = self.item_def(item).abilities.get(i).and_then(|a| a.on_stack()).map(|(r, ..)| r).unwrap_or("");
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
        let zone = if from == Zone::Stack && self.obj(id).flashback { Zone::Exile } else { zone };
        self.obj_mut(id).flashback = false;
        if !(from == Zone::Stack && zone == Zone::Battlefield) {
            self.obj_mut(id).escaped = false;
        }
        // Rest in Peace (two of them: the player chooses which applies)
        let rest_in_peace = self
            .battlefield
            .iter()
            .filter(|src| {
                self.obj(**src).def.abilities.iter().any(|a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => {
                        effects.iter().any(|e| matches!(e, EffectDef::GraveyardFromAnywhereExileReplacementEffect))
                    }
                    _ => false,
                })
            })
            .count();
        let zone = if zone == Zone::Graveyard && rest_in_peace > 0 {
            if rest_in_peace > 1 {
                let o = self.obj(id);
                self.owed_choices.push(if from == Zone::Battlefield { o.controller } else { o.owner });
            }
            Zone::Exile
        } else {
            zone
        };
        // Etching of Kumano: a creature dealt damage this turn by a source its controller controlled is exiled
        let etching = from == Zone::Battlefield && zone == Zone::Graveyard && self.has_type(id, CardType::Creature) && {
            let (turn, mask) = self.obj(id).damaged_by;
            turn == self.turn
                && self.battlefield.iter().any(|b| {
                    let s = self.obj(*b);
                    mask & (1 << s.controller) != 0
                        && s.def.abilities.iter().any(|a| match a {
                            AbilityDef::SimpleStaticAbility { effects, .. } => {
                                effects.iter().any(|e| matches!(e, EffectDef::EtchingOfKumanoReplacementEffect))
                            }
                            _ => false,
                        })
                })
        };
        // "if it would die this turn, exile it instead"
        let zone = if from == Zone::Battlefield
            && zone == Zone::Graveyard
            && (etching || self.exile_if_dies.contains(&(id, self.obj(id).zcc)) || self.obj(id).exile_instead_of_dying)
        {
            Zone::Exile
        } else {
            zone
        };
        // dies triggers look back at the battlefield as it was (this one included), and at what left it in the same
        // action (Java's short-living LKI: a sweeper's creatures see each other die)
        let died_controller = self.obj(id).controller;
        if from == Zone::Battlefield && zone == Zone::Graveyard && self.has_type(id, CardType::Creature) {
            self.creature_died = true;
        }
        let dies = if from == Zone::Battlefield && zone == Zone::Graveyard {
            let fired;
            let mut sources: Vec<ObjId> = self.lookback.iter().copied().filter(|o| !self.battlefield.contains(o)).collect();
            sources.extend(self.battlefield.iter().copied());
            fired = self.triggers(sources, |g, src, you, t| match t {
                Trigger::Dies | Trigger::EntersOrDies | Trigger::LeavesBattlefield => src == id,
                Trigger::DiesIfCreature => src == id && g.has_type(id, CardType::Creature),
                // the source itself always counts ("this or another"); others must match
                Trigger::DiesAll { filter, include_self } => {
                    if src == id {
                        *include_self
                    } else {
                        g.filter_matches(filter, you, Some(src), id)
                    }
                }
                Trigger::PeltCollector => src != id && died_controller == you,
                Trigger::LeavesAll { filter } => g.filter_matches(filter, you, Some(src), id),
                _ => false,
            });
            let mut fired = self.pelt_collector(fired, id);
            // blitz: its granted "When this creature dies, draw a card."
            if self.obj(id).blitzed {
                let delayed = Delayed {
                    rule: "When this creature dies, draw a card.".into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: vec![EffectDef::DrawCardSourceControllerEffect { amount: crate::card::Amount::Fixed(1) }],
                    source_zcc: self.obj(id).zcc,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                };
                fired.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, died_controller) });
            }
            fired
        } else if from == Zone::Battlefield && zone != Zone::Battlefield {
            let sources: Vec<ObjId> = self.battlefield.iter().copied().collect();
            self.triggers(sources, |g, src, you, t| match t {
                Trigger::LeavesBattlefield => src == id,
                Trigger::LeavesAll { filter } => g.filter_matches(filter, you, Some(src), id),
                _ => false,
            })
        } else {
            Vec::new()
        };
        let p1p1 = self.obj(id).counters.iter().find(|(c, _)| c == "+1/+1").map_or(0, |c| c.1);
        let power = self.power_toughness(id).0;
        let mut dies = dies;
        for t in dies.iter_mut() {
            if matches!(self.trigger_of(t), Some(Trigger::LeavesAll { .. })) {
                t.x = p1p1;
            }
            // its own leave triggers remember its power as it last was (`SourcePermanentPowerValue`)
            if t.card == id && t.lki_power.is_none() {
                t.lki_power = Some(power);
            }
        }
        // earthbend's delayed trigger (delayed triggers are checked before the others)
        if from == Zone::Battlefield && matches!(zone, Zone::Graveyard | Zone::Exile) {
            let zcc = self.obj(id).zcc;
            if let Some(k) = self.earthbent.iter().position(|e| *e == (id, zcc)) {
                self.earthbent.remove(k);
                let delayed = Delayed {
                    rule: "When it dies or is exiled, return it to the battlefield tapped.".into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: Vec::new(),
                    source_zcc: 0,
                    fixed: Vec::new(),
                    return_tapped: vec![(id, zcc + 1)],
                };
                let owner = self.obj(id).owner;
                dies.insert(0, StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, owner) });
            }
        }
        if from == Zone::Battlefield && zone != Zone::Battlefield {
            let zcc = self.obj(id).zcc;
            let (back, kept): (Vec<_>, Vec<_>) =
                self.exiled_until_leaves.iter().copied().partition(|(s, z, _)| *s == id && *z == zcc);
            self.exiled_until_leaves = kept;
            let controller = self.obj(id).controller;
            self.returning.extend(back.into_iter().map(|(_, _, c)| (controller, c)));
        }
        if from == Zone::Battlefield && zone != Zone::Battlefield {
            let c = self.obj(id).controller;
            self.player_mut(c).permanent_left = true;
        }
        // what lost its abilities while this was on the battlefield gets them back
        if from == Zone::Battlefield {
            let zcc = self.obj(id).zcc;
            for b in self.battlefield.to_vec() {
                if self.obj(b).lost_abilities.as_ref().is_some_and(|(s, z, _)| *s == id && *z == zcc) {
                    let o = self.obj_mut(b);
                    if let Some((_, _, def)) = o.lost_abilities.take() {
                        o.def = def;
                    }
                }
            }
            let o = self.obj_mut(id);
            if let Some((_, _, def)) = o.lost_abilities.take() {
                o.def = def;
            }
        }
        self.remove_from_zone(id);
        {
            let o = self.obj_mut(id);
            if from == Zone::Stack {
                if let Some(def) = o.adventurer.take() {
                    o.def = def;
                }
            }
            if from == Zone::Battlefield {
                if let Some(def) = o.front.take() {
                    o.def = def;
                }
                o.damaged_by = (0, 0);
            }
            o.on_adventure = false;
            o.exile_instead_of_dying = false;
            o.may_play = None;
            o.play_tax = 0;
            o.exerted = false;
            o.powered_up = false;
            o.creature_types = None;
            o.base_pt = None;
            o.regen_shields = 0;
            o.warp_from = None;
            if !(from == Zone::Stack && zone == Zone::Battlefield) {
                o.warped = false;
                o.blitzed = false;
                o.offspring = false;
            }
        }
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
                o.cast = false;
                o.attached_to = None;
                if let Some(def) = o.until_end_of_turn_def.take() {
                    o.def = def;
                }
                if let Some(def) = o.printed_def.take() {
                    o.def = def;
                }
                o.cast_x = 0;
                o.blocking.clear();
                o.blocks = 0;
                o.blocked = false;
                o.blocked_by.clear();
                o.controller = o.owner;
                o.deathtouched = false;
            }
            if from != Zone::Battlefield && !matches!(zone, Zone::Battlefield | Zone::Stack) {
                o.adventure_zone = zone;
            }
            o.zone = zone;
            o.zcc += 1;
        }
        for mut t in dies {
            // a dead permanent's own trigger is its graveyard card's
            if t.card == id {
                t.ability = t.ability.map(|(i, _)| (i, self.obj(id).zcc));
            }
            // "that player": the dead permanent's controller
            let a = t.ability.and_then(|(i, _)| self.item_def(&t).abilities.get(i));
            if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. })) {
                t.targets = vec![vec![Target::Player(died_controller)]];
                t.target_zcc = vec![vec![0]];
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
