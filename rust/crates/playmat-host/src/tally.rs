//! The Career's tally of a hosted game (`CareerTally.Recorder`): what the human did, counted from the engine's
//! events as Java's watcher counts its game's. Java gets it with the game's end (`hostGameEnded`) and pays the game as
//! one it played: levels, quests, achievements, masteries (docs/rust/GameHost.md, step F). It's counted once, at the
//! end, from the game's whole history: what was taken back (a land play that wasn't paid for) isn't in it.

use playmat_engine::card::{CardType, Color};
use playmat_engine::state::{EventKind, Game, GameEvent, Seat, Target, Zone};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// `CareerTally.MOMENT_THRESHOLDS`: a feat counts from this big
const MOMENT_THRESHOLDS: [(&str, i32); 6] =
    [("big_hit", 10), ("storm", 5), ("mana_surge", 10), ("life_swing", 10), ("elimination", 1), ("commander_hit", 7)];

/// One seat's tally
#[derive(Clone, Debug)]
pub struct Tally {
    seat: Seat,
    counters: BTreeMap<String, i32>,
    /// feats in the order they first counted: id, value, turn
    moments: Vec<(String, i32, u32)>,
    spells_this_turn: i32,
    mana_this_turn: i32,
    lowest_life: i32,
    /// the life the events have taken the seat to since the last look at the game
    life: Option<i32>,
}

impl Tally {
    /// A seat's tally from the game's start, at `starting_life`
    pub fn new(seat: Seat, starting_life: i32) -> Tally {
        Tally {
            seat,
            counters: BTreeMap::new(),
            moments: Vec::new(),
            spells_this_turn: 0,
            mana_this_turn: 0,
            lowest_life: i32::MAX,
            life: Some(starting_life),
        }
    }

    fn add(&mut self, counter: &str, amount: i32) {
        if amount != 0 {
            *self.counters.entry(counter.to_string()).or_default() += amount;
        }
    }

    fn max(&mut self, counter: &str, value: i32) {
        let c = self.counters.entry(counter.to_string()).or_insert(value);
        *c = (*c).max(value);
    }

    fn get(&self, counter: &str) -> i32 {
        self.counters.get(counter).copied().unwrap_or(0)
    }

    fn moment(&mut self, id: &str, value: i32, turn: u32) {
        let threshold = MOMENT_THRESHOLDS.iter().find(|(m, _)| *m == id).map(|(_, t)| *t);
        if threshold.is_some_and(|t| value < t) {
            return;
        }
        match self.moments.iter_mut().find(|(m, _, _)| m == id) {
            Some(best) if value > best.1 => *best = (id.to_string(), value, turn),
            Some(_) => {}
            None => self.moments.push((id.to_string(), value, turn)),
        }
    }

    fn life_now(&mut self, life: i32) {
        self.lowest_life = self.lowest_life.min(life);
    }

    /// The events since the last batch, `game` as it is after them
    pub fn watch(&mut self, game: &Game, events: &[GameEvent]) {
        let me = self.seat;
        let mine = |s: Option<Seat>| s == Some(me);
        // the life the events take the seat through (Java looks at it after each one)
        let mut life = self.life.unwrap_or(game.player(me).life);
        for e in events {
            match e.kind {
                EventKind::Turn => {
                    self.spells_this_turn = 0;
                    self.mana_this_turn = 0;
                    if mine(e.player) {
                        self.add("my_turns", 1);
                    }
                }
                EventKind::Cast if mine(e.player) => {
                    let Some(Target::Obj(spell)) = e.target else { continue };
                    self.spell(game, spell, e.turn);
                    if game.obj(spell).cast_from == Some(Zone::Graveyard) {
                        self.add("cast_from_graveyard", 1);
                    }
                }
                EventKind::Countered => {
                    if let Some(src) = e.source {
                        if game.obj(src).controller == me && !mine(e.player) {
                            self.add("spells_countered", 1);
                        }
                    }
                }
                EventKind::Loyalty if mine(e.player) => self.add("loyalty_abilities", 1),
                EventKind::Attached if mine(e.player) => {
                    if let Some(a) = e.source {
                        if game.has_subtype(a, "Aura") {
                            self.add("auras_attached", 1);
                        } else if game.has_subtype(a, "Equipment") {
                            self.add("equipment_attached", 1);
                        }
                    }
                }
                EventKind::Sacrificed if mine(e.player) => self.add("sacrificed", 1),
                EventKind::Discarded if e.player.is_some() && !mine(e.player) => self.add("opponent_discards", 1),
                EventKind::Milled if e.player.is_some() && !mine(e.player) => self.add("opponent_milled", e.amount),
                EventKind::Mana if mine(e.player) => {
                    self.mana_this_turn += e.amount;
                    let mana = self.mana_this_turn;
                    self.max("most_mana_in_a_turn", mana);
                    self.moment("mana_surge", mana, e.turn);
                }
                EventKind::Damage => {
                    let from_me = e.source.is_some_and(|s| game.obj(s).controller == me);
                    match e.target {
                        Some(Target::Player(p)) if p == me => {
                            // (the life it costs comes as its own LifeLoss)
                            self.add("damage_taken", e.amount);
                        }
                        Some(Target::Player(_)) if from_me => {
                            self.add("damage_to_opponents", e.amount);
                            let how = if e.combat { "combat_damage_to_opponents" } else { "noncombat_damage_to_opponents" };
                            self.add(how, e.amount);
                            self.max("biggest_hit", e.amount);
                            self.moment("big_hit", e.amount, e.turn);
                            if let Some(src) = e.source.filter(|_| e.combat) {
                                if game.has_type(src, CardType::Creature) {
                                    self.max("max_attacker_power", game.power_toughness(src).0);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                EventKind::LifeLoss if e.target == Some(Target::Player(me)) => {
                    life -= e.amount;
                    self.life_now(life);
                }
                EventKind::LifeGain if e.target == Some(Target::Player(me)) => {
                    life += e.amount;
                    self.add("life_gained", e.amount);
                    self.moment("life_swing", e.amount, e.turn);
                }
                EventKind::Land if mine(e.player) => self.add("lands_played", 1),
                EventKind::Draw if mine(e.player) => self.add("cards_drawn", 1),
                EventKind::Attack if mine(e.player) => self.add("attacks", 1),
                EventKind::Block if mine(e.player) => self.add("blocks", 1),
                EventKind::Counters => {
                    if let Some(Target::Obj(o)) = e.target {
                        if game.obj(o).controller == me && game.obj(o).zone == Zone::Battlefield {
                            self.add("counters_added", e.amount);
                        }
                    }
                }
                EventKind::Zone => self.zone_change(game, e),
                _ => {}
            }
        }
        // the game's own life from here on: what no event says (a life total set, damage from a host
        // that wasn't recorded) is in it
        self.life = Some(game.player(me).life);
    }

    fn spell(&mut self, game: &Game, spell: playmat_engine::state::ObjId, turn: u32) {
        self.add("spells", 1);
        self.spells_this_turn += 1;
        let n = self.spells_this_turn;
        self.max("most_spells_in_a_turn", n);
        self.moment("storm", n, turn);
        let colors = game.colors(spell);
        for (c, key) in [
            (Color::W, "spells_W"),
            (Color::U, "spells_U"),
            (Color::B, "spells_B"),
            (Color::R, "spells_R"),
            (Color::G, "spells_G"),
        ] {
            if colors.contains(&c) {
                self.add(key, 1);
            }
        }
        if colors.is_empty() {
            self.add("spells_colorless", 1);
        } else if colors.len() > 1 {
            self.add("spells_multicolor", 1);
        }
        for (t, key) in [
            (CardType::Creature, "creature_spells"),
            (CardType::Enchantment, "enchantment_spells"),
            (CardType::Artifact, "artifact_spells"),
            (CardType::Planeswalker, "planeswalker_spells"),
        ] {
            if game.has_type(spell, t) {
                self.add(key, 1);
            }
        }
        if game.has_type(spell, CardType::Instant) || game.has_type(spell, CardType::Sorcery) {
            self.add("instant_sorcery_spells", 1);
        }
        let mv = game.obj(spell).def.mana_value() as i32;
        if mv >= 5 {
            self.add("spells_mv5", 1);
        }
        if mv <= 2 {
            self.add("spells_cheap", 1);
        }
    }

    fn zone_change(&mut self, game: &Game, e: &GameEvent) {
        let me = self.seat;
        let Some(Target::Obj(o)) = e.target else { return };
        let obj = game.obj(o);
        let creature = obj.def.is(CardType::Creature);
        if e.to == Some(Zone::Battlefield) && creature && obj.controller == me {
            self.add("creatures_entered", 1);
        }
        if e.from == Some(Zone::Battlefield) && e.to == Some(Zone::Graveyard) && creature && obj.controller != me {
            self.add("opponent_creatures_died", 1);
        }
        if e.to == Some(Zone::Exile) && e.from != Some(Zone::Stack) && obj.owner != me {
            if e.source.is_some_and(|s| game.obj(s).controller == me) {
                self.add("cards_exiled", 1);
            }
        }
    }

    /// The tally for Java (`CareerTally.fromHost`), with what only the end tells: the seat's life, the
    /// opponents' life, poison and library
    pub fn report(&self, game: &Game) -> Value {
        let me = self.seat;
        let mut counters: Map<String, Value> = self.counters.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        let tokens = game.objs.iter().filter(|o| o.token && o.owner == me).count() as i32;
        if tokens > 0 {
            counters.insert("tokens_created".into(), json!(tokens));
        }
        let opponents: Vec<Value> = (0..game.seats())
            .filter(|s| *s != me)
            .map(|s| {
                let p = game.player(s);
                let poison: i32 = p.counters.iter().filter(|(n, _)| n == "poison").map(|(_, k)| *k).sum();
                json!({"life": p.life, "poison": poison, "library": p.library.len()})
            })
            .collect();
        let life = game.player(me).life;
        json!({
            "counters": counters,
            "moments": self.moments.iter().map(|(id, v, t)| json!({"id": id, "value": v, "turn": t})).collect::<Vec<_>>(),
            "lowestLife": self.lowest_life.min(life),
            "life": life,
            "startingLife": game.starting_life,
            "opponents": opponents,
        })
    }

    /// a counter's value so far
    pub fn counter(&self, name: &str) -> i32 {
        self.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playmat_ai::duel::{new_duel, Deck};
    use playmat_ai::search::{Config, Table};
    use playmat_engine::rules;

    fn deck(file: &str) -> Deck {
        let path = format!("{}/../playmat-engine/shadow-decks/tier30/{file}", env!("CARGO_MANIFEST_DIR"));
        Deck::parse(file, &std::fs::read_to_string(path).unwrap())
    }

    #[test]
    fn an_ai_game_tallies_like_javas_watcher() {
        let db = playmat_engine::cards();
        let decks = [deck("gw-company.txt"), deck("b-zombies.txt")];
        let mut game = new_duel(&db, [&decks[0], &decks[1]], 5, 0).unwrap();
        game.events = Some(Vec::new());
        let quick = Config { budget: 200, width: 3, ..Config::default() };
        let mut table = Table::new(vec![Some(quick.clone()), Some(quick)]);
        rules::play_from_priority(&mut game, &mut table, 30);
        for seat in 0..2 {
            let mut t = Tally::new(seat, 20);
            t.watch(&game, game.events.as_deref().unwrap());
            let r = t.report(&game);
            let n = |k: &str| r["counters"][k].as_i64().unwrap_or(0);
            assert!(n("my_turns") > 0 && n("lands_played") > 0, "{r}");
            assert!(n("spells") > 0, "an AI casts spells: {r}");
            let colored: i64 = ["W", "U", "B", "R", "G"].iter().map(|c| n(&format!("spells_{c}"))).sum();
            assert!(colored + n("spells_colorless") >= n("spells") - n("spells_multicolor"), "{r}");
            assert!(n("most_mana_in_a_turn") > 0 && n("attacks") > 0, "{r}");
            assert!(n("most_spells_in_a_turn") >= 1 && n("most_spells_in_a_turn") <= n("spells"), "{r}");
            let life = game.player(seat).life;
            assert!(r["lowestLife"].as_i64().unwrap() <= life as i64, "{r}");
        }
    }
}
