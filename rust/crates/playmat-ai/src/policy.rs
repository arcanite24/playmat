//! The quick answers: what a seat says when nothing is searched. Searches play their games out with these, for
//! both seats, so they must be cheap and never get stuck (the engine asks again after an answer it can't use, and
//! the search's answer budget ends any loop).

use crate::eval;
use playmat_engine::card::{CardType, ManaCost};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::rules;
use playmat_engine::state::{Game, ObjId, Seat, Step, Zone};
use std::collections::HashMap;

/// Effects whose text says they hurt what they target
const HARMFUL: [&str; 16] = [
    "destroy",
    "exile",
    "damage",
    "to its owner's hand",
    "tap target",
    "counter target",
    "sacrifice",
    "fights",
    "fight ",
    "loses",
    "lose ",
    "discard",
    "gets -",
    "get -",
    "-1/-1",
    "can't block",
];

#[derive(Default)]
pub struct Policy {
    /// whether a spell or ability (by its card, and ability) hurts its targets
    harmful: HashMap<(usize, Option<usize>), bool>,
    /// blockers declared this combat, so a refused block isn't tried again
    tried_blockers: Vec<ObjId>,
    blocker: Option<ObjId>,
    /// the game at the last priority question, roughly, and what was played then: a play that comes straight back
    /// to the same priority (a cancelled cast) isn't tried again until something changes
    last: Option<(Progress, Option<ObjId>)>,
    banned: Vec<ObjId>,
    /// the stronger answers (`Config::playout` 2): instant-speed play, abilities, removal and burn by the effect
    v2: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct Progress([u64; 9]);

fn progress(game: &Game, seat: Seat) -> Progress {
    let p = game.player(seat);
    let tapped = game.battlefield.iter().filter(|id| game.obj(**id).tapped).count() as u64;
    Progress([
        game.turn as u64,
        game.step.map(|s| s as u64).unwrap_or(99),
        game.stack.len() as u64,
        game.battlefield.len() as u64,
        p.hand.len() as u64,
        p.graveyard.len() as u64,
        p.mana_pool.total() as u64,
        tapped,
        (p.life as i64 + 1000) as u64 * 1000 + (game.player(game.opponent(seat)).life as i64 + 1000) as u64,
    ])
}

impl Policy {
    /// The quick answers of a `Config::playout` version (1, or 2 for the stronger ones)
    pub fn version(v: u8) -> Policy {
        Policy { v2: v >= 2, ..Policy::default() }
    }

    pub fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        match q {
            Query::Priority { playable } => {
                self.tried_blockers.clear();
                let playable = self.playable(game, seat, playable);
                let a = self.priority(game, seat, &playable);
                self.played(&a);
                a
            }
            Query::PlayMana { cost, sources } => pay(game, seat, cost, sources),
            Query::Target { players, objs, required } => self.target(game, seat, players, objs, *required),
            Query::Attackers { creatures } => attack(game, seat, creatures),
            Query::Blockers { creatures } => self.block(game, seat, creatures),
            Query::BlockTarget { attackers } => self.block_target(game, attackers),
            Query::Cards { objs, required } => cards(game, seat, objs, *required),
            Query::MultiAmount { defaults, .. } => {
                Answer::Text(defaults.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(" "))
            }
            Query::ChooseAbility { source, abilities } if self.v2 => loyalty_ability(game, seat, *source, abilities)
                .or_else(|| {
                    // activated at priority, not paying for something: an ability that does more than make mana
                    let paying = game.unpaid.is_some() || game.mana_paid.is_some();
                    let def = &game.obj(*source).def;
                    (!paying)
                        .then(|| abilities.iter().copied().find(|i| def.abilities.get(*i).is_some_and(|a| a.mana().is_none())))
                        .flatten()
                        .map(|i| Answer::Ability(*source, i))
                })
                .unwrap_or_else(|| choose_ability(game, seat, *source, abilities)),
            Query::ChooseAbility { source, abilities } => choose_ability(game, seat, *source, abilities),
            Query::PickTrigger { .. } => Answer::Trigger(0),
            Query::YesNo { .. } => Answer::Bool(true),
            Query::ChooseMode { modes } => modes.first().map(|m| Answer::Mode(*m)).unwrap_or(Answer::ModeDone),
            Query::Amount { min, max } => Answer::Int(x_amount(game, seat).clamp(*min, (*max).max(*min))),
            Query::Choice { message, choices } => choice(game, seat, message, choices),
        }
    }

    /// The playable objects less those that came straight back last time. Call once per priority question.
    pub fn playable(&mut self, game: &Game, seat: Seat, playable: &[ObjId]) -> Vec<ObjId> {
        let now = progress(game, seat);
        match self.last.take() {
            Some((before, Some(played))) if before == now => self.banned.push(played),
            Some((before, _)) if before == now => {}
            _ => self.banned.clear(),
        }
        self.last = Some((now, None));
        playable.iter().copied().filter(|id| !self.banned.contains(id)).collect()
    }

    /// What was answered at the priority question `playable` was called for.
    pub fn played(&mut self, a: &Answer) {
        if let Some((_, played)) = self.last.as_mut() {
            *played = match a {
                Answer::Obj(id) | Answer::Ability(id, _) => Some(*id),
                _ => None,
            };
        }
    }

    /// A land first, then the costliest spell in hand, in a main phase of the seat's own turn with nothing on the
    /// stack; otherwise pass. Abilities of permanents are left to the search.
    pub fn priority(&mut self, game: &Game, seat: Seat, playable: &[ObjId]) -> Answer {
        if self.v2 {
            return self.priority_v2(game, seat, playable);
        }
        let main = game.step.is_some_and(|s| s.is_main());
        if game.active != Some(seat) || !main || !game.stack.is_empty() {
            return Answer::Bool(false);
        }
        let in_hand: Vec<ObjId> = playable.iter().copied().filter(|id| game.obj(*id).zone == Zone::Hand).collect();
        if let Some(land) = in_hand.iter().find(|id| game.obj(**id).def.types.contains(&CardType::Land)) {
            return Answer::Obj(*land);
        }
        in_hand
            .iter()
            .chain(playable.iter().filter(|id| matches!(game.obj(**id).zone, Zone::Graveyard | Zone::Exile)))
            .max_by_key(|id| game.obj(**id).def.mana_value())
            .map(|id| Answer::Obj(*id))
            .unwrap_or(Answer::Bool(false))
    }

    /// Whether what is being cast or put on the stack (the top) hurts its targets.
    fn harmful(&mut self, game: &Game) -> bool {
        let Some(top) = game.stack.last() else { return true };
        let key = (top.card.0 as usize, top.ability.map(|(i, _)| i));
        if let Some(h) = self.harmful.get(&key) {
            return *h;
        }
        let def = game.item_def(top);
        let text = match top.ability {
            Some((i, _)) => def.abilities.get(i).map(|a| format!("{:?}", a)).unwrap_or_default().to_lowercase(),
            None => def.spell.rule.to_lowercase(),
        };
        let h = HARMFUL.iter().any(|w| text.contains(w))
            || ["destroy", "exile", "damage", "bounce", "tap", "counterunless", "sacrifice", "fight", "loselife"]
                .iter()
                .any(|w| text.contains(w));
        self.harmful.insert(key, h);
        h
    }

    fn target(&mut self, game: &Game, seat: Seat, players: &[Seat], objs: &[ObjId], required: bool) -> Answer {
        // nothing left to choose ("up to one" with no legal target): done
        if players.is_empty() && objs.is_empty() {
            return Answer::Bool(false);
        }
        let opp = game.opponent(seat);
        let harmful = self.harmful(game);
        if self.v2 {
            if let Some(a) = self.target_v2(game, seat, players, objs, harmful) {
                return a;
            }
        }
        let value = |id: &ObjId| {
            let o = game.obj(*id);
            if o.zone == Zone::Battlefield {
                eval::permanent_score(game, *id)
            } else {
                eval::hand_card_score(game, *id)
            }
        };
        let whose = |id: &ObjId| game.obj(*id).controller;
        if harmful {
            if let Some(best) = objs.iter().filter(|o| whose(o) == opp).max_by_key(|o| value(o)) {
                return Answer::Obj(*best);
            }
            if players.contains(&opp) {
                return Answer::Player(opp);
            }
        } else {
            if let Some(best) = objs.iter().filter(|o| whose(o) == seat).max_by_key(|o| value(o)) {
                return Answer::Obj(*best);
            }
            if players.contains(&seat) {
                return Answer::Player(seat);
            }
        }
        let _ = required;
        // only the wrong side (the AI picks whenever it can: a done means different things to Java's dialogs): the least the choice costs
        if harmful {
            objs.iter().min_by_key(|o| value(o)).map(|o| Answer::Obj(*o))
        } else {
            objs.iter().max_by_key(|o| if whose(o) == seat { value(o) } else { -value(o) }).map(|o| Answer::Obj(*o))
        }
        .or_else(|| players.first().map(|p| Answer::Player(*p)))
        .unwrap_or(Answer::Stop)
    }

    /// Blocks that kill the attacker or survive it, and chump blocks when the attack is lethal.
    fn block(&mut self, game: &Game, seat: Seat, creatures: &[ObjId]) -> Answer {
        let attackers = rules::attackers(game);
        let unblocked: Vec<ObjId> = attackers.iter().copied().filter(|a| !game.obj(*a).blocked).collect();
        let incoming: i32 = unblocked.iter().map(|a| game.power_toughness(*a).0.max(0)).sum();
        let lethal = incoming >= game.player(seat).life;
        for b in creatures {
            if self.tried_blockers.contains(b) || !game.obj(*b).blocking.is_empty() {
                continue;
            }
            let (bp, bt) = game.power_toughness(*b);
            let good = unblocked.iter().copied().filter(|a| rules::can_block(game, *b, *a)).find(|a| {
                let (ap, at) = game.power_toughness(*a);
                bt > ap || (bp >= at && eval::permanent_score(game, *a) >= eval::permanent_score(game, *b))
            });
            let chump = if lethal {
                unblocked
                    .iter()
                    .copied()
                    .filter(|a| rules::can_block(game, *b, *a))
                    .max_by_key(|a| game.power_toughness(*a).0)
            } else {
                None
            };
            if let Some(a) = good.or(chump) {
                self.tried_blockers.push(*b);
                self.blocker = Some(a);
                return Answer::Obj(*b);
            }
        }
        Answer::Bool(true)
    }

    fn block_target(&mut self, game: &Game, attackers: &[ObjId]) -> Answer {
        match self.blocker.take() {
            Some(a) if attackers.contains(&a) => Answer::Obj(a),
            _ => attackers
                .iter()
                .max_by_key(|a| game.power_toughness(**a).0)
                .map(|a| Answer::Obj(*a))
                .unwrap_or(Answer::Bool(false)),
        }
    }
}

/// Attacks with a creature no untapped opposing creature can block and kill, or with everything when that's lethal.
fn attack(game: &Game, seat: Seat, creatures: &[ObjId]) -> Answer {
    let opp = game.opponent(seat);
    let blockers: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| {
            let o = game.obj(*b);
            o.controller == opp && !o.tapped && game.has_type(*b, CardType::Creature)
        })
        .collect();
    let power: i32 =
        creatures.iter().chain(rules::attackers(game).iter()).map(|c| game.power_toughness(*c).0.max(0)).sum();
    if blockers.is_empty() && !creatures.is_empty() {
        return Answer::Text("special".into());
    }
    if power >= game.player(opp).life + blockers.len() as i32 * 2 && !creatures.is_empty() {
        return Answer::Text("special".into());
    }
    for c in creatures {
        let (cp, ct) = game.power_toughness(*c);
        if cp <= 0 {
            continue;
        }
        let dies = blockers.iter().any(|b| rules::can_block(game, *b, *c) && game.power_toughness(*b).0 >= ct);
        if !dies {
            return Answer::Obj(*c);
        }
    }
    Answer::Bool(true)
}

/// A mana source that makes a color still owed, else any; Cancel when none helps.
fn pay(game: &Game, seat: Seat, cost: &str, sources: &[ObjId]) -> Answer {
    let owed = ManaCost::parse(cost).unwrap_or_default();
    let makes = |id: &ObjId| -> [u32; 6] {
        let mut m = [0u32; 6];
        for a in game.obj(*id).def.abilities.iter() {
            if let Some(mana) = a.mana() {
                for (k, n) in mana.colored.iter().enumerate() {
                    m[k] += n;
                }
                m[5] += mana.colorless;
            }
        }
        m
    };
    let mut mine: Vec<ObjId> = sources.iter().copied().filter(|s| game.obj(*s).controller == seat).collect();
    // the permanent whose ability this pays for doesn't tap for it while another source can (Mutavault animated
    // and then tapped for its own {1} can't attack)
    if let Some(top) = game.stack.last().filter(|t| t.ability.is_some()) {
        if mine.len() > 1 {
            mine.retain(|s| *s != top.card);
        }
    }
    // a color owed: the source making the fewest other colors, so flexible lands stay for later
    for (k, n) in owed.colored.iter().enumerate() {
        if *n > 0 {
            if let Some(s) =
                mine.iter().filter(|s| makes(s)[k] > 0).min_by_key(|s| makes(s).iter().filter(|n| **n > 0).count())
            {
                return Answer::Obj(*s);
            }
        }
    }
    if owed.colorless > 0 {
        if let Some(s) = mine.iter().find(|s| makes(s)[5] > 0) {
            return Answer::Obj(*s);
        }
    }
    if owed.snow > 0 {
        // a snow symbol: only a snow permanent's mana pays it
        if let Some(s) = mine.iter().find(|s| game.is_snow(**s)) {
            return Answer::Obj(*s);
        }
    }
    if owed.generic > 0 || owed.x > 0 {
        // generic: the least useful source (colorless first, then the fewest colors)
        if let Some(s) = mine.iter().min_by_key(|s| {
            let m = makes(s);
            // lands that can become creatures pay last, after creatures
            let animates = game.obj(**s).def.abilities.iter().enumerate().any(|(i, _)| ability_kind(&game.obj(**s).def, i).animate);
            (
                animates as usize,
                m[..5].iter().filter(|n| **n > 0).count(),
                game.has_type(**s, CardType::Creature) as usize,
            )
        }) {
            return Answer::Obj(*s);
        }
    }
    Answer::Bool(false)
}

/// Of a permanent's abilities: for mana, one making a color the spell being paid for needs.
fn choose_ability(game: &Game, _seat: Seat, source: ObjId, abilities: &[usize]) -> Answer {
    let def = &game.obj(source).def;
    let need = game.stack.last().map(|t| game.item_def(t).cost.colored).unwrap_or_default();
    let best = abilities.iter().copied().find(|i| {
        def.abilities.get(*i).and_then(|a| a.mana()).is_some_and(|m| (0..5).any(|k| m.colored[k] > 0 && need[k] > 0))
    });
    Answer::Ability(source, best.or(abilities.first().copied()).unwrap_or(0))
}

/// Cards from the seat's own hand go least valuable first (discards); other cards most valuable first.
fn cards(game: &Game, seat: Seat, objs: &[ObjId], required: bool) -> Answer {
    let own_hand = objs.iter().all(|o| game.obj(*o).zone == Zone::Hand && game.obj(*o).owner == seat);
    if !required && own_hand {
        return Answer::Bool(false);
    }
    let pick = if own_hand {
        objs.iter().min_by_key(|o| eval::hand_card_score(game, **o))
    } else {
        objs.iter().max_by_key(|o| {
            let v = if game.obj(**o).zone == Zone::Battlefield {
                eval::permanent_score(game, **o)
            } else {
                eval::hand_card_score(game, **o)
            };
            if game.obj(**o).controller == seat {
                v
            } else {
                -v
            }
        })
    };
    pick.map(|o| Answer::Obj(*o)).unwrap_or(Answer::Bool(false))
}

/// X: what the seat's untapped mana sources make beyond the rest of the spell's cost.
fn x_amount(game: &Game, seat: Seat) -> i32 {
    let sources = game
        .battlefield
        .iter()
        .filter(|id| {
            let o = game.obj(**id);
            o.controller == seat && !o.tapped && o.def.abilities.iter().any(|a| a.mana().is_some())
        })
        .count() as i32
        + game.player(seat).mana_pool.total() as i32;
    let rest = game
        .stack
        .last()
        .map(|t| game.item_def(t).cost.clone())
        .map(|c| (c.generic + c.colorless + c.colored.iter().sum::<u32>()) as i32)
        .unwrap_or(0);
    let per = game.stack.last().map(|t| game.item_def(t).cost.x.max(1) as i32).unwrap_or(1);
    ((sources - rest) / per).max(0)
}

fn choice(game: &Game, seat: Seat, message: &str, choices: &[String]) -> Answer {
    if !choices.is_empty() {
        // a color of mana: one the spell being paid for needs
        let need = game.stack.last().map(|t| game.item_def(t).cost.colored).unwrap_or_default();
        let names = ["White", "Blue", "Black", "Red", "Green"];
        for (k, n) in names.iter().enumerate() {
            if need[k] > 0 {
                if let Some(c) = choices.iter().find(|c| c.eq_ignore_ascii_case(n)) {
                    return Answer::Text(c.clone());
                }
            }
        }
        return Answer::Text(choices[0].clone());
    }
    if message.contains("creature type") {
        // the type most of the seat's creatures share
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for o in game.objs.iter().filter(|o| o.owner == seat && o.def.types.contains(&CardType::Creature)) {
            for s in o.def.subtypes.iter() {
                *counts.entry(s.as_str()).or_default() += 1;
            }
        }
        let best = counts.into_iter().max_by_key(|(n, c)| (*c, std::cmp::Reverse(*n))).map(|(n, _)| n.to_string());
        return Answer::Text(best.unwrap_or_else(|| "Human".into()));
    }
    if message.contains("Land card name") {
        return Answer::Text("Island".into());
    }
    Answer::Text(String::new())
}

/// Catches a question asked again since the last priority question (an answer the engine ignored, one that toggles,
/// or two questions taking turns): each repeat gets an answer not given to it yet, then one that ends it.
#[derive(Default)]
pub struct Repeats {
    asked: Vec<(Seat, Query, Vec<Answer>)>,
}

impl Repeats {
    /// Whether `q` was asked since the last priority question
    pub fn is_repeat(&self, seat: Seat, q: &Query) -> bool {
        self.asked.iter().any(|(s, l, _)| *s == seat && l == q)
    }

    pub fn guard(&mut self, seat: Seat, q: &Query, proposed: Answer, options: &[Answer]) -> Answer {
        // a priority question coming back is normal (each play comes back to it): the policy's own check handles it
        if matches!(q, Query::Priority { .. }) {
            self.asked.clear();
            return proposed;
        }
        let Some(i) = self.asked.iter().position(|(s, l, _)| *s == seat && l == q) else {
            self.asked.push((seat, q.clone(), vec![proposed.clone()]));
            return proposed;
        };
        let given = &mut self.asked[i].2;
        let a = std::iter::once(proposed)
            .chain(options.iter().cloned())
            .find(|a| !given.contains(a))
            .unwrap_or_else(|| finish(q, given.len(), options));
        given.push(a.clone());
        a
    }
}

/// An answer that ends a question: done, cancel, or (asked too often) stop.
fn finish(q: &Query, tries: usize, options: &[Answer]) -> Answer {
    if tries > 40 {
        return Answer::Stop;
    }
    match q {
        Query::Attackers { .. } | Query::Blockers { .. } => Answer::Bool(true),
        Query::ChooseMode { .. } => Answer::ModeDone,
        Query::Amount { min, .. } => Answer::Int(*min),
        Query::Choice { choices, .. } => choices.first().map(|c| Answer::Text(c.clone())).unwrap_or(Answer::Stop),
        // a required choice can't be left: pick again (Java asks until the choice is made)
        Query::Target { required: true, .. } | Query::Cards { required: true, .. } => {
            options.get(tries % options.len().max(1)).cloned().unwrap_or(Answer::Stop)
        }
        _ => Answer::Bool(false),
    }
}

/// What a card's spell (or an ability) does, as far as the quick answers care: read from its rule text and the names
/// of its effects
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Kind {
    /// counters a spell
    pub counter: bool,
    /// destroys, exiles, damages, shrinks, bounces or fights a creature
    pub removal: bool,
    /// fixed damage it deals, and whether a player can be its target
    pub damage: Option<i32>,
    pub face: bool,
    /// makes a creature bigger or better for the turn
    pub pump: bool,
    /// draws or looks for cards
    pub draw: bool,
    /// a land or permanent that becomes a creature (Mutavault)
    pub animate: bool,
    /// destroys or damages every creature
    pub sweeper: bool,
}

/// The kind of a card's spell
pub fn spell_kind(def: &playmat_engine::card::CardDef) -> Kind {
    kind_of(&def.spell.rule.to_lowercase(), &format!("{:?}", def.spell.effects))
}

/// The kind of a card's ability, by its index
pub fn ability_kind(def: &playmat_engine::card::CardDef, i: usize) -> Kind {
    match def.abilities.get(i) {
        Some(a) => {
            let debug = format!("{a:?}");
            let rule = a.activated().map(|(r, ..)| r.to_lowercase()).unwrap_or_default();
            kind_of(&rule, &debug)
        }
        None => Kind::default(),
    }
}

fn kind_of(rule: &str, effects: &str) -> Kind {
    let damage = rule.find(" deals ").and_then(|at| {
        let rest = &rule[at + 7..];
        let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        n.parse::<i32>().ok().filter(|_| rest.contains("damage"))
    });
    let face = damage.is_some()
        && (rule.contains("any target")
            || rule.contains("target player")
            || rule.contains("target opponent")
            || rule.contains("each opponent"));
    let sweeper = rule.contains("destroy all creatures")
        || rule.contains("each creature")
        || effects.contains("DestroyAllEffect")
        || effects.contains("DamageAllEffect");
    Kind {
        counter: rule.contains("counter target") || effects.contains("CounterTargetEffect"),
        removal: !sweeper
            && (rule.contains("destroy target")
                || rule.contains("exile target")
                || (damage.is_some() && (rule.contains("target creature") || rule.contains("any target")))
                || rule.contains("target creature an opponent controls gets -")
                || rule.contains("target creature gets -")
                || rule.contains("fights")
                || rule.contains("to its owner's hand")
                || effects.contains("DestroyTargetEffect")
                || effects.contains("ExileTargetEffect")
                || effects.contains("FightTargetsEffect")),
        damage,
        face,
        pump: (rule.contains("target creature gets +") || effects.contains("BoostTargetEffect"))
            && !rule.contains("gets -"),
        draw: rule.contains("draw") || rule.contains("look at the top"),
        animate: effects.contains("BecomesCreatureSourceEffect"),
        sweeper,
    }
}

impl Policy {
    /// The stronger quick answers (`Config::playout` 2):
    /// - with an opponent's spell on the stack: a counterspell when the spell is worth one, else pass;
    /// - in its own main phase with an empty stack: a land, burn to the face when that's lethal, its most expensive
    ///   spell (counters, tricks and sweepers that would hit more of its own creatures held back), a planeswalker's
    ///   loyalty ability, an equip, and before combat a land that becomes a creature when it can attack;
    /// - in combat: removal on an attacker, a pump on a blocked attacker or on a blocker;
    /// - at the end of the opponent's turn: flash creatures, removal and card draw with the mana left.
    fn priority_v2(&mut self, game: &Game, seat: Seat, playable: &[ObjId]) -> Answer {
        let opp = game.opponent(seat);
        let mine = game.active == Some(seat);
        let step = game.step;
        let own: Vec<ObjId> = playable
            .iter()
            .copied()
            .filter(|id| {
                let o = game.obj(*id);
                o.controller == seat || o.owner == seat
            })
            .collect();
        let castable: Vec<ObjId> =
            own.iter().copied().filter(|id| matches!(game.obj(*id).zone, Zone::Hand | Zone::Graveyard | Zone::Exile)).collect();
        let kind = |id: ObjId| spell_kind(&game.obj(id).def);
        let is = |id: ObjId, t: CardType| game.obj(id).def.types.contains(&t);
        let instant_speed =
            |id: ObjId| is(id, CardType::Instant) || game.obj(id).def.abilities.iter().any(|a| matches!(a, playmat_engine::card::AbilityDef::FlashAbility));
        let creatures = |s: Seat| -> Vec<ObjId> {
            game.battlefield
                .iter()
                .copied()
                .filter(|id| game.obj(*id).controller == s && game.has_type(*id, CardType::Creature))
                .collect()
        };

        // the stack: answer an opponent's spell, or let it resolve
        if let Some(top) = game.stack.last() {
            if top.controller == opp && top.ability.is_none() {
                let spell = &game.obj(top.card).def;
                let worth = spell.mana_value() >= 2 || spell_kind(spell).removal || spell_kind(spell).sweeper;
                if worth {
                    if let Some(c) = castable.iter().copied().find(|c| kind(*c).counter) {
                        return Answer::Obj(c);
                    }
                }
            }
            return Answer::Bool(false);
        }

        let combat = matches!(step, Some(Step::DeclareAttackers) | Some(Step::DeclareBlockers));
        if combat {
            let attackers = rules::attackers(game);
            if !mine && !attackers.is_empty() {
                // removal on an attacker
                if let Some(c) = castable.iter().copied().find(|c| instant_speed(*c) && kind(*c).removal) {
                    return Answer::Obj(c);
                }
            }
            if step == Some(Step::DeclareBlockers) {
                let fighting = attackers.iter().any(|a| {
                    let o = game.obj(*a);
                    (mine && o.blocked) || (!mine && !o.blocked_by.is_empty())
                });
                if fighting {
                    if let Some(c) = castable.iter().copied().find(|c| instant_speed(*c) && kind(*c).pump) {
                        return Answer::Obj(c);
                    }
                }
            }
            return Answer::Bool(false);
        }

        if !mine {
            // the opponent's end step: use the mana left
            if step == Some(Step::EndTurn) {
                let opp_creatures = !creatures(opp).is_empty();
                let pick = castable
                    .iter()
                    .copied()
                    .filter(|c| instant_speed(*c) && !is(*c, CardType::Land))
                    .filter(|c| {
                        let k = kind(*c);
                        (k.removal && opp_creatures) || k.draw || is(*c, CardType::Creature)
                    })
                    .max_by_key(|c| game.obj(*c).def.mana_value());
                if let Some(c) = pick {
                    return Answer::Obj(c);
                }
            }
            return Answer::Bool(false);
        }

        // its own turn, outside combat
        if !step.is_some_and(|s| s.is_main()) {
            return Answer::Bool(false);
        }
        if let Some(land) = castable.iter().copied().find(|id| is(*id, CardType::Land) && game.obj(*id).zone == Zone::Hand) {
            return Answer::Obj(land);
        }
        // burn to the face when it's lethal
        let opp_life = game.player(opp).life;
        let burn: Vec<ObjId> = castable.iter().copied().filter(|c| kind(*c).face && kind(*c).damage.is_some()).collect();
        let total: i32 = burn.iter().map(|c| kind(*c).damage.unwrap_or(0)).sum();
        if total >= opp_life {
            if let Some(c) = burn.iter().copied().max_by_key(|c| kind(*c).damage) {
                return Answer::Obj(c);
            }
        }
        let my_count = creatures(seat).len();
        let opp_count = creatures(opp).len();
        let spell = castable
            .iter()
            .copied()
            .filter(|c| {
                let k = kind(*c);
                if is(*c, CardType::Land) || k.counter {
                    return false;
                }
                // tricks wait for combat
                if k.pump && !is(*c, CardType::Creature) {
                    return false;
                }
                // a sweeper only when it costs the opponent more creatures than it costs us
                if k.sweeper && opp_count <= my_count {
                    return false;
                }
                // removal needs something to remove
                if k.removal && !k.face && opp_count == 0 && !is(*c, CardType::Creature) {
                    return false;
                }
                true
            })
            .max_by_key(|c| game.obj(*c).def.mana_value());
        if let Some(c) = spell {
            return Answer::Obj(c);
        }
        // permanents' abilities: planeswalkers, equipment, lands that become creatures before combat
        for id in own.iter().copied().filter(|id| game.obj(*id).zone == Zone::Battlefield) {
            let o = game.obj(id);
            if o.def.types.contains(&CardType::Planeswalker) && o.loyalty_used == 0 {
                return Answer::Obj(id);
            }
            if game.has_subtype(id, "Equipment") && game.attached(id).is_none() && my_count > 0 {
                return Answer::Obj(id);
            }
            let animates = o.def.abilities.iter().enumerate().any(|(i, _)| ability_kind(&o.def, i).animate);
            if animates
                && step == Some(Step::PrecombatMain)
                && !game.has_type(id, CardType::Creature)
                && !o.tapped
                && game.turn > o.entered_turn
            {
                return Answer::Obj(id);
            }
        }
        Answer::Bool(false)
    }

    /// Targets the stronger answers pick differently: lethal burn to the face, damage to a creature it kills (the
    /// best such, attacking first), and in combat help for its own creature that is fighting. `None`: as before.
    fn target_v2(&mut self, game: &Game, seat: Seat, players: &[Seat], objs: &[ObjId], harmful: bool) -> Option<Answer> {
        let opp = game.opponent(seat);
        let top = game.stack.last()?;
        let def = game.item_def(top);
        let k = match top.ability {
            Some((i, _)) => ability_kind(def, i),
            None => spell_kind(def),
        };
        let theirs: Vec<ObjId> = objs.iter().copied().filter(|o| game.obj(*o).controller == opp).collect();
        if harmful {
            if let Some(n) = k.damage {
                if players.contains(&opp) && n >= game.player(opp).life {
                    return Some(Answer::Player(opp));
                }
                // the best creature the damage kills, an attacker first
                let kills = theirs
                    .iter()
                    .copied()
                    .filter(|o| game.obj(*o).zone == Zone::Battlefield && game.has_type(*o, CardType::Creature))
                    .filter(|o| game.power_toughness(*o).1 - game.obj(*o).damage <= n)
                    .max_by_key(|o| (game.obj(*o).attacking.is_some(), eval::permanent_score(game, *o)));
                if let Some(o) = kills {
                    return Some(Answer::Obj(o));
                }
                if players.contains(&opp) {
                    return Some(Answer::Player(opp));
                }
            }
            // in combat, an attacker first
            let attacking = theirs
                .iter()
                .copied()
                .filter(|o| game.obj(*o).attacking.is_some())
                .max_by_key(|o| eval::permanent_score(game, *o));
            return attacking.map(Answer::Obj);
        }
        // help: its own creature in a fight
        let fighting = objs
            .iter()
            .copied()
            .filter(|o| {
                let x = game.obj(*o);
                x.controller == seat && (x.attacking.is_some() || !x.blocking.is_empty())
            })
            .max_by_key(|o| {
                let x = game.obj(*o);
                (x.blocked || !x.blocking.is_empty(), eval::permanent_score(game, *o))
            });
        fighting.map(Answer::Obj)
    }
}

/// A planeswalker's loyalty ability: the minus one that keeps it alive when the opponent has a creature for it to
/// remove, else its plus ability, else the first
fn loyalty_ability(game: &Game, seat: Seat, source: ObjId, abilities: &[usize]) -> Option<Answer> {
    let o = game.obj(source);
    if !o.def.types.contains(&CardType::Planeswalker) {
        return None;
    }
    let loyalty = o.counter("loyalty");
    let cost = |i: usize| -> Option<i32> {
        match o.def.abilities.get(i)? {
            playmat_engine::card::AbilityDef::LoyaltyAbility { cost, .. } => cost.iter().find_map(|c| match c {
                playmat_engine::card::CostDef::PayLoyaltyCost { amount } => Some(*amount),
                _ => None,
            }),
            _ => None,
        }
    };
    let opp = game.opponent(seat);
    let opp_creatures =
        game.battlefield.iter().any(|id| game.obj(*id).controller == opp && game.has_type(*id, CardType::Creature));
    let minus = abilities
        .iter()
        .copied()
        .filter(|i| cost(*i).is_some_and(|c| c < 0 && loyalty + c > 0))
        .filter(|i| ability_kind(&o.def, *i).removal && opp_creatures)
        .min_by_key(|i| cost(*i));
    let plus = abilities.iter().copied().filter(|i| cost(*i).is_some_and(|c| c >= 0)).max_by_key(|i| cost(*i));
    minus.or(plus).or(abilities.first().copied()).map(|i| Answer::Ability(source, i))
}
