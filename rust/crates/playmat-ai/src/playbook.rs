//! Forge-inspired declarative AI Playbooks: archetype base profiles (Layer A) + per-deck scripted hints (Layer B).
//!
//! A playbook steers `Policy` (priority timing, hold-up mana discipline, modal picks, discards, combat role protection),
//! `fitted` (archetype feature multipliers and key-card bonuses), `mulligan` (curve/color/land bounds), and `search`
//! (candidate ordering) with zero per-deck Rust branching and O(1) evaluation during playouts.

use playmat_engine::card::CardType;
use playmat_engine::state::{Game, ObjId, Seat, Step, Zone};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MulliganHints {
    pub min_lands: Option<usize>,
    pub max_lands: Option<usize>,
    pub require_early_action_by_turn: Option<u32>,
    pub require_color_match: Option<bool>,
    pub prefer_keep_roles: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TimingHints {
    pub hold_mana_for_instants: Option<bool>,
    pub hold_mana_priority: Option<u8>,
    pub cast_creatures_main2: Option<bool>,
    pub min_counter_mv: Option<u32>,
    pub sweeper_min_net_creatures: Option<i32>,
    pub sweeper_emergency_life: Option<i32>,
    pub burn_face_threshold_life: Option<i32>,
    pub use_eot_draw: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CombatHints {
    pub aggression: Option<i32>,
    pub protect_roles: Vec<String>,
    pub chump_block_life_threshold: Option<i32>,
    pub block_evasive_priority: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldStep {
    OppEndStep,
    CombatAttackers,
    CombatBlockers,
    Main2,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Condition {
    OppSpellOnStack(bool),
    OppSpellMvAtLeast(u32),
    Step(String),
    MyTurn(bool),
    NotMyTurn(bool),
    ManaAvailableAtLeast(u32),
    MyLandsAtMost(usize),
    NoLandInHand(bool),
    MyLifeAtMost(i32),
    OppLifeAtMost(i32),
    OppCreaturesAtLeast(usize),
    OppPowerAtLeast(i32),
    NetOppCreaturesAtLeast(i32),
    GraveyardHasRole(String),
    BattlefieldHasRole(String),
    HandHasRole(String),
    OppUntappedBlockersAtMost(usize),
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModeRule {
    #[serde(rename = "if")]
    pub when: Option<Condition>,
    pub default: bool,
    pub modes: Vec<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CardRule {
    pub card: String,
    pub hold_for_step: Option<HoldStep>,
    pub unless: Option<Condition>,
    pub cast_only_if: Option<Condition>,
    pub avoid_if: Option<Condition>,
    pub mode_priority: Vec<ModeRule>,
    pub sac_priority: Vec<String>,
    pub never_sac_roles: Vec<String>,
    pub tutor_priority: Vec<String>,
    pub prefer_before_combat: bool,
    pub prefer_main2: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RawProfile {
    id: String,
    mulligan: MulliganHints,
    timing: TimingHints,
    combat: CombatHints,
    eval_multipliers: HashMap<String, f64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RawDeck {
    profile: Option<String>,
    archetype: Option<String>,
    mulligan: MulliganHints,
    timing: TimingHints,
    combat: CombatHints,
    eval_multipliers: HashMap<String, f64>,
    eval_card_bonus: HashMap<String, f64>,
    graveyard_role_bonus: HashMap<String, f64>,
    roles: HashMap<String, Vec<String>>,
    play_order: Vec<String>,
    discard_roles_first: Vec<String>,
    never_discard_roles: Vec<String>,
    card_rules: Vec<CardRule>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct RawBundle {
    profiles: HashMap<String, RawProfile>,
    decks: HashMap<String, RawDeck>,
}

/// Compiled, fast-to-query runtime playbook attached to an AI seat.
#[derive(Clone, Debug)]
pub struct Playbook {
    pub id: String,
    pub archetype: String,
    pub min_lands: usize,
    pub max_lands: usize,
    pub require_early_action_by_turn: Option<u32>,
    pub require_color_match: bool,
    pub prefer_keep_roles: Vec<String>,
    pub hold_mana_for_instants: bool,
    pub hold_mana_priority: u8,
    pub cast_creatures_main2: bool,
    pub min_counter_mv: u32,
    pub sweeper_min_net_creatures: i32,
    pub sweeper_emergency_life: i32,
    pub burn_face_threshold_life: i32,
    pub use_eot_draw: bool,
    pub aggression: i32,
    pub protect_roles: Vec<String>,
    pub chump_block_life_threshold: i32,
    pub block_evasive_priority: f64,
    /// Multipliers for `fitted::NAMES` (default 1.0)
    pub eval_weights: [f64; 17],
    pub eval_card_bonus: HashMap<String, f64>,
    pub graveyard_role_bonus: HashMap<String, f64>,
    pub roles: HashMap<String, Vec<String>>,
    pub play_order: Vec<String>,
    pub discard_roles_first: Vec<String>,
    pub never_discard_roles: Vec<String>,
    pub card_rules: HashMap<String, CardRule>,
}

impl Default for Playbook {
    fn default() -> Self {
        Playbook {
            id: "default".into(),
            archetype: "midrange".into(),
            min_lands: 2,
            max_lands: 5,
            require_early_action_by_turn: None,
            require_color_match: false,
            prefer_keep_roles: Vec::new(),
            hold_mana_for_instants: false,
            hold_mana_priority: 0,
            cast_creatures_main2: false,
            min_counter_mv: 2,
            sweeper_min_net_creatures: 1,
            sweeper_emergency_life: 7,
            burn_face_threshold_life: 6,
            use_eot_draw: true,
            aggression: 0,
            protect_roles: Vec::new(),
            chump_block_life_threshold: 6,
            block_evasive_priority: 1.0,
            eval_weights: [1.0; 17],
            eval_card_bonus: HashMap::new(),
            graveyard_role_bonus: HashMap::new(),
            roles: HashMap::new(),
            play_order: Vec::new(),
            discard_roles_first: Vec::new(),
            never_discard_roles: Vec::new(),
            card_rules: HashMap::new(),
        }
    }
}

impl Playbook {
    fn from_profile(id: String, p: &RawProfile) -> Self {
        let mut pb = Playbook { id, archetype: p.id.clone(), ..Playbook::default() };
        pb.apply_mulligan(&p.mulligan);
        pb.apply_timing(&p.timing);
        pb.apply_combat(&p.combat);
        pb.apply_multipliers(&p.eval_multipliers);
        pb
    }

    fn apply_mulligan(&mut self, m: &MulliganHints) {
        if let Some(v) = m.min_lands {
            self.min_lands = v;
        }
        if let Some(v) = m.max_lands {
            self.max_lands = v;
        }
        if m.require_early_action_by_turn.is_some() {
            self.require_early_action_by_turn = m.require_early_action_by_turn;
        }
        if let Some(v) = m.require_color_match {
            self.require_color_match = v;
        }
        if !m.prefer_keep_roles.is_empty() {
            self.prefer_keep_roles = m.prefer_keep_roles.clone();
        }
    }

    fn apply_timing(&mut self, t: &TimingHints) {
        if let Some(v) = t.hold_mana_for_instants {
            self.hold_mana_for_instants = v;
        }
        if let Some(v) = t.hold_mana_priority {
            self.hold_mana_priority = v;
        }
        if let Some(v) = t.cast_creatures_main2 {
            self.cast_creatures_main2 = v;
        }
        if let Some(v) = t.min_counter_mv {
            self.min_counter_mv = v;
        }
        if let Some(v) = t.sweeper_min_net_creatures {
            self.sweeper_min_net_creatures = v;
        }
        if let Some(v) = t.sweeper_emergency_life {
            self.sweeper_emergency_life = v;
        }
        if let Some(v) = t.burn_face_threshold_life {
            self.burn_face_threshold_life = v;
        }
        if let Some(v) = t.use_eot_draw {
            self.use_eot_draw = v;
        }
    }

    fn apply_combat(&mut self, c: &CombatHints) {
        if let Some(v) = c.aggression {
            self.aggression = v;
        }
        if !c.protect_roles.is_empty() {
            self.protect_roles = c.protect_roles.clone();
        }
        if let Some(v) = c.chump_block_life_threshold {
            self.chump_block_life_threshold = v;
        }
        if let Some(v) = c.block_evasive_priority {
            self.block_evasive_priority = v;
        }
    }

    fn apply_multipliers(&mut self, map: &HashMap<String, f64>) {
        for (k, v) in map {
            if let Some(idx) = crate::fitted::NAMES.iter().position(|n| n == k) {
                self.eval_weights[idx] = *v;
            }
        }
    }

    /// Whether `card_name` carries `role` in this playbook.
    pub fn has_role(&self, card_name: &str, role: &str) -> bool {
        self.roles.get(card_name).is_some_and(|rs| rs.iter().any(|r| r == role))
    }

    /// Whether `card_name` carries any role in `list`.
    pub fn has_any_role(&self, card_name: &str, list: &[String]) -> bool {
        self.roles.get(card_name).is_some_and(|rs| rs.iter().any(|r| list.iter().any(|want| want == r)))
    }

    /// Whether a creature should be protected in combat (not traded away lightly).
    pub fn is_protected(&self, card_name: &str) -> bool {
        self.has_any_role(card_name, &self.protect_roles)
    }

    /// Play-order rank (lower = cast earlier); `usize::MAX` when unranked.
    pub fn play_rank(&self, card_name: &str) -> usize {
        let Some(rs) = self.roles.get(card_name) else { return usize::MAX };
        rs.iter().filter_map(|r| self.play_order.iter().position(|p| p == r)).min().unwrap_or(usize::MAX)
    }

    pub fn rule(&self, card_name: &str) -> Option<&CardRule> {
        self.card_rules.get(card_name)
    }

    /// Evaluates a declarative condition predicate against the current game state.
    pub fn eval_cond(&self, cond: &Condition, game: &Game, seat: Seat) -> bool {
        let opp = game.opponent(seat);
        match cond {
            Condition::OppSpellOnStack(want) => {
                let has = game.stack.last().is_some_and(|t| t.controller == opp && t.ability.is_none());
                has == *want
            }
            Condition::OppSpellMvAtLeast(mv) => game.stack.last().is_some_and(|t| {
                t.controller == opp && t.ability.is_none() && game.obj(t.card).def.mana_value() >= *mv
            }),
            Condition::Step(name) => game.step.is_some_and(|s| format!("{s:?}").eq_ignore_ascii_case(name)),
            Condition::MyTurn(want) => (game.active == Some(seat)) == *want,
            Condition::NotMyTurn(want) => (game.active != Some(seat)) == *want,
            Condition::ManaAvailableAtLeast(n) => available_mana(game, seat) >= *n,
            Condition::MyLandsAtMost(n) => {
                let lands = game
                    .battlefield
                    .iter()
                    .filter(|id| game.obj(**id).controller == seat && game.has_type(**id, CardType::Land))
                    .count();
                lands <= *n
            }
            Condition::NoLandInHand(want) => {
                let none = !game.player(seat).hand.iter().any(|id| game.obj(*id).is(CardType::Land));
                none == *want
            }
            Condition::MyLifeAtMost(n) => game.player(seat).life <= *n,
            Condition::OppLifeAtMost(n) => game.player(opp).life <= *n,
            Condition::OppCreaturesAtLeast(n) => count_creatures(game, opp) >= *n,
            Condition::OppPowerAtLeast(n) => total_power(game, opp) >= *n,
            Condition::NetOppCreaturesAtLeast(n) => {
                count_creatures(game, opp) as i32 - count_creatures(game, seat) as i32 >= *n
            }
            Condition::GraveyardHasRole(role) => {
                game.player(seat).graveyard.iter().any(|id| self.has_role(&game.obj(*id).def.name, role))
            }
            Condition::BattlefieldHasRole(role) => game
                .battlefield
                .iter()
                .any(|id| game.obj(*id).controller == seat && self.has_role(&game.obj(*id).def.name, role)),
            Condition::HandHasRole(role) => {
                game.player(seat).hand.iter().any(|id| self.has_role(&game.obj(*id).def.name, role))
            }
            Condition::OppUntappedBlockersAtMost(n) => {
                let blockers = game
                    .battlefield
                    .iter()
                    .filter(|id| {
                        let o = game.obj(**id);
                        o.controller == opp && !o.tapped && game.has_type(**id, CardType::Creature)
                    })
                    .count();
                blockers <= *n
            }
            Condition::And(list) => list.iter().all(|c| self.eval_cond(c, game, seat)),
            Condition::Or(list) => list.iter().any(|c| self.eval_cond(c, game, seat)),
            Condition::Not(inner) => !self.eval_cond(inner, game, seat),
        }
    }

    /// Whether a card's rule says to hold it back right now (during a main phase or non-target step).
    pub fn should_hold_card(&self, card_name: &str, game: &Game, seat: Seat) -> bool {
        let Some(rule) = self.card_rules.get(card_name) else { return false };
        if let Some(cond) = &rule.cast_only_if {
            if !self.eval_cond(cond, game, seat) {
                return true;
            }
        }
        if let Some(cond) = &rule.avoid_if {
            if self.eval_cond(cond, game, seat) {
                return true;
            }
        }
        if let Some(hold) = rule.hold_for_step {
            if let Some(unless) = &rule.unless {
                if self.eval_cond(unless, game, seat) {
                    return false;
                }
            }
            let mine = game.active == Some(seat);
            let step = game.step;
            let allowed = match hold {
                HoldStep::OppEndStep => !mine && step == Some(Step::EndTurn),
                HoldStep::CombatAttackers => {
                    matches!(step, Some(Step::DeclareAttackers) | Some(Step::DeclareBlockers))
                }
                HoldStep::CombatBlockers => step == Some(Step::DeclareBlockers),
                HoldStep::Main2 => mine && step == Some(Step::PostcombatMain),
            };
            if !allowed {
                return true;
            }
        }
        false
    }

    /// Preferred modal index (if any) from the card's `modePriority` rules.
    pub fn preferred_mode(&self, card_name: &str, game: &Game, seat: Seat, available: &[usize]) -> Option<usize> {
        let rule = self.card_rules.get(card_name)?;
        for entry in &rule.mode_priority {
            let matches = match &entry.when {
                Some(cond) => self.eval_cond(cond, game, seat),
                None => entry.default,
            };
            if matches {
                for m in &entry.modes {
                    if available.contains(m) {
                        return Some(*m);
                    }
                }
            }
        }
        None
    }

    /// Mana to keep open in Main 1/2 if this playbook practices instant-speed discipline and hand holds a castable instant.
    pub fn reserve_mana(&self, game: &Game, seat: Seat, castable: &[ObjId]) -> u32 {
        if !self.hold_mana_for_instants {
            return 0;
        }
        let mut need = 0u32;
        for id in castable {
            let o = game.obj(*id);
            if o.zone != Zone::Hand {
                continue;
            }
            let name = o.def.name.as_str();
            let is_hold_instant = self.has_role(name, "counter")
                || self.has_role(name, "eot_instant")
                || self
                    .card_rules
                    .get(name)
                    .is_some_and(|r| matches!(r.hold_for_step, Some(HoldStep::OppEndStep | HoldStep::CombatAttackers)));
            if is_hold_instant {
                let mv = o.def.mana_value();
                if mv > 0 && mv <= available_mana(game, seat) {
                    need = need.max(mv);
                }
            }
        }
        need
    }
}

pub fn available_mana(game: &Game, seat: Seat) -> u32 {
    let untapped = game
        .battlefield
        .iter()
        .filter(|id| {
            let o = game.obj(**id);
            o.controller == seat && !o.tapped && o.def.abilities.iter().any(|a| a.mana().is_some())
        })
        .count() as u32;
    untapped + game.player(seat).mana_pool.total()
}

fn count_creatures(game: &Game, seat: Seat) -> usize {
    game.battlefield
        .iter()
        .filter(|id| game.obj(**id).controller == seat && game.has_type(**id, CardType::Creature))
        .count()
}

fn total_power(game: &Game, seat: Seat) -> i32 {
    game.battlefield
        .iter()
        .filter(|id| game.obj(**id).controller == seat && game.has_type(**id, CardType::Creature))
        .map(|id| game.power_toughness(*id).0.max(0))
        .sum()
}

struct Registry {
    profiles: HashMap<String, Arc<Playbook>>,
    decks: HashMap<String, Arc<Playbook>>,
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let raw: RawBundle = serde_json::from_str(include_str!("playbooks.json")).expect("playbooks.json");
        let mut profiles = HashMap::new();
        for (id, p) in &raw.profiles {
            profiles.insert(id.clone(), Arc::new(Playbook::from_profile(format!("profile:{id}"), p)));
        }
        let mut decks = HashMap::new();
        for (id, d) in &raw.decks {
            let base = d
                .profile
                .as_deref()
                .and_then(|k| raw.profiles.get(k))
                .map(|p| Playbook::from_profile(id.clone(), p))
                .unwrap_or_else(|| Playbook { id: id.clone(), ..Playbook::default() });
            let mut pb = base;
            if let Some(arch) = &d.archetype {
                pb.archetype = arch.clone();
            }
            pb.apply_mulligan(&d.mulligan);
            pb.apply_timing(&d.timing);
            pb.apply_combat(&d.combat);
            pb.apply_multipliers(&d.eval_multipliers);
            pb.eval_card_bonus = d.eval_card_bonus.clone();
            pb.graveyard_role_bonus = d.graveyard_role_bonus.clone();
            pb.roles = d.roles.clone();
            pb.play_order = d.play_order.clone();
            pb.discard_roles_first = d.discard_roles_first.clone();
            pb.never_discard_roles = d.never_discard_roles.clone();
            for rule in &d.card_rules {
                pb.card_rules.insert(rule.card.clone(), rule.clone());
            }
            decks.insert(id.clone(), Arc::new(pb));
        }
        Registry { profiles, decks }
    })
}

/// Resolves a playbook spec (`"profile:control"`, `"standard/meta-azorius-control"`, or with optional `@hash`).
pub fn resolve(spec: Option<&str>) -> Option<Arc<Playbook>> {
    let raw = spec?.trim();
    if raw.is_empty() || raw == "-" {
        return None;
    }
    let key = raw.split('@').next().unwrap_or(raw);
    let reg = registry();
    if let Some(name) = key.strip_prefix("profile:") {
        return reg.profiles.get(name).cloned();
    }
    reg.decks.get(key).cloned().or_else(|| reg.profiles.get(key).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duel::{new_duel, Deck};

    #[test]
    fn resolves_profiles_and_deck_playbooks() {
        let ctrl = resolve(Some("profile:control")).expect("control profile");
        assert!(ctrl.hold_mana_for_instants);
        assert_eq!(ctrl.min_lands, 3);
        assert!(ctrl.eval_weights[9] > 1.5); // hand weight

        let az = resolve(Some("standard/meta-azorius-control@v1")).expect("azorius playbook");
        assert_eq!(az.archetype, "control");
        assert!(az.has_role("No More Lies", "counter"));
        assert!(az.rule("Three Steps Ahead").is_some());

        let goryo = resolve(Some("modern/ladder-meta-goryos")).expect("goryos playbook");
        assert!(goryo.discard_roles_first.iter().any(|r| r == "reanimate_target"));
    }

    #[test]
    fn evaluates_hold_and_condition_rules() {
        let db = playmat_engine::cards();
        let d = Deck::parse("a", "20 Plains\n4 Day of Judgment");
        let mut game = new_duel(&db, [&d, &d], 1, 0).unwrap();
        let pb = resolve(Some("standard/meta-azorius-control")).unwrap();
        game.active = Some(0);
        game.step = Some(Step::PrecombatMain);
        // Empty board, full life: Day of Judgment should be held
        assert!(pb.should_hold_card("Day of Judgment", &game, 0));
        // Low life emergency: Day of Judgment allowed
        game.player_mut(0).life = 8;
        assert!(!pb.should_hold_card("Day of Judgment", &game, 0));
    }

    #[test]
    fn reanimator_discards_reanimate_target_first() {
        use crate::policy::Policy;
        use playmat_engine::decide::{Answer, Query};
        let db = playmat_engine::cards();
        let d = Deck::parse("g", "20 Swamp\n4 Atraxa, Grand Unifier\n4 Thoughtseize\n4 Goryo's Vengeance");
        let mut game = new_duel(&db, [&d, &d], 1, 0).unwrap();
        let atraxa = game.objs.iter().find(|o| o.owner == 0 && o.def.name == "Atraxa, Grand Unifier").unwrap().id;
        let ts = game.objs.iter().find(|o| o.owner == 0 && o.def.name == "Thoughtseize").unwrap().id;
        game.move_to(atraxa, Zone::Hand);
        game.move_to(ts, Zone::Hand);
        let pb = resolve(Some("modern/ladder-meta-goryos")).unwrap();
        let mut pol = Policy::with_playbook(2, Some(pb));
        let ans = pol.answer(&game, 0, &Query::Cards { objs: vec![ts, atraxa], required: true });
        assert_eq!(ans, Answer::Obj(atraxa), "reanimator discards its reanimate target instead of Thoughtseize");
    }

    #[test]
    fn control_holds_up_counter_mana_and_picks_modes() {
        use crate::policy::Policy;
        use playmat_engine::decide::{Answer, Query};
        let db = playmat_engine::cards();
        let d = Deck::parse("c", "20 Island\n20 Plains\n4 No More Lies\n4 Stock Up\n4 Three Steps Ahead");
        let mut game = new_duel(&db, [&d, &d], 1, 0).unwrap();
        // Put 3 untapped lands on battlefield, No More Lies (MV 2) and Stock Up (MV 3) in hand
        let mut placed = 0;
        for id in
            game.objs.iter().filter(|o| o.owner == 0 && o.def.is(CardType::Land)).map(|o| o.id).collect::<Vec<_>>()
        {
            if placed < 3 {
                game.move_to(id, Zone::Battlefield);
                placed += 1;
            }
        }
        let nml = game.objs.iter().find(|o| o.owner == 0 && o.def.name == "No More Lies").unwrap().id;
        let stock = game.objs.iter().find(|o| o.owner == 0 && o.def.name == "Stock Up").unwrap().id;
        game.move_to(nml, Zone::Hand);
        game.move_to(stock, Zone::Hand);
        game.active = Some(0);
        game.step = Some(Step::PrecombatMain);
        let pb = resolve(Some("standard/meta-azorius-control")).unwrap();
        let mut pol = Policy::with_playbook(2, Some(pb));
        // With 3 mana available and No More Lies (MV 2) in hand, tapping out for Stock Up (MV 3) leaves 0 < 2 -> hold!
        let ans = pol.answer(&game, 0, &Query::Priority { playable: vec![nml, stock] });
        assert_eq!(ans, Answer::Bool(false), "control holds up counter mana instead of tapping out");
    }

    #[test]
    fn pin_round_trips_with_playbook_and_reads_legacy_gen1_pins() {
        use crate::entrants::{label, pinned};
        use crate::search::Config;
        let cfg = Config { playbook: Some("standard/meta-azorius-control".into()), ..Config::default() };
        let text = label(&cfg);
        assert!(text.contains("playbook=standard/meta-azorius-control"));
        let back = pinned(&text).unwrap().unwrap();
        assert_eq!(back.playbook.as_deref(), Some("standard/meta-azorius-control"));
        // Gen 1 pin without playbook= still parses cleanly with playbook: None
        let gen1 = "budget=6000+width=12+extra_turns=1+playout_cap=1500+policy=playouts+hide=0+samples=1+playout=2+eval=2+plans=0+mulligan_sim=0+blunder=0+net=builtin+temp=0+eta=10000+rollout=-+consider=4+cpuct=300+iters=256";
        let legacy = pinned(gen1).unwrap().unwrap();
        assert!(legacy.playbook.is_none());
    }
}
