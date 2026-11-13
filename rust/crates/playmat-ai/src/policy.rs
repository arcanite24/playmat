//! The quick answers: what a seat says when nothing is searched. Searches play their games out with these, for
//! both seats, so they must be cheap and never get stuck (the engine asks again after an answer it can't use, and
//! the search's answer budget ends any loop).

use crate::eval;
use playmat_engine::card::{CardType, ManaCost};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::rules;
use playmat_engine::state::{Game, ObjId, Seat, Zone};
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
    let mine: Vec<ObjId> = sources.iter().copied().filter(|s| game.obj(*s).controller == seat).collect();
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
            (m[..5].iter().filter(|n| **n > 0).count(), game.has_type(**s, CardType::Creature) as usize)
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
