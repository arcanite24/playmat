//! Whole combat plans (`Config::plans`): a search compares complete attacks and complete blocks, not one creature at
//! a time with the quick answers finishing the declaration. A plan is the answers to give in order: the creatures to
//! toggle (blockers each followed by the attacker they block, when the engine will ask which), then done.

use crate::eval;
use crate::policy::incoming_player_damage;
use playmat_engine::card::AbilityDef;
use playmat_engine::decide::{Answer, Query};
use playmat_engine::rules;
use playmat_engine::state::{Game, ObjId};
use std::collections::HashMap;

/// Most plans a search compares
pub const MAX_PLANS: usize = 24;

/// The plans worth comparing for an attack or block question, the quick answer alone first (the quick answers then
/// finish the declaration). `None` for other questions.
pub fn plans(game: &Game, q: &Query, quick: &Answer) -> Option<Vec<Vec<Answer>>> {
    let mut out: Vec<Vec<Answer>> = vec![vec![quick.clone()]];
    let mut add = |p: Vec<Answer>| {
        if out.len() < MAX_PLANS && !out.contains(&p) {
            out.push(p);
        }
    };
    match q {
        Query::Attackers { creatures } => {
            let m: Vec<ObjId> = creatures.iter().copied().filter(|c| game.power_toughness(*c).0 > 0).collect();
            let plan = |set: &[ObjId]| {
                let mut p: Vec<Answer> = set.iter().map(|c| Answer::Obj(*c)).collect();
                p.push(Answer::Bool(true));
                p
            };
            add(plan(&[]));
            if m.len() <= 4 {
                for bits in 1u32..(1 << m.len()) {
                    let set: Vec<ObjId> = (0..m.len()).filter(|i| bits & (1 << i) != 0).map(|i| m[i]).collect();
                    add(plan(&set));
                }
            } else {
                add(plan(&m));
                for i in 0..m.len() {
                    add(plan(&[m[i]]));
                    let rest: Vec<ObjId> = m.iter().copied().filter(|c| *c != m[i]).collect();
                    add(plan(&rest));
                }
            }
        }
        Query::Blockers { creatures } => {
            let attackers = rules::attackers(game);
            // each blocker's choices: no block, or one of the attackers it can block
            let options: Vec<(ObjId, Vec<ObjId>)> = creatures
                .iter()
                .filter(|b| game.obj(**b).blocking.is_empty())
                .map(|b| (*b, attackers.iter().copied().filter(|a| rules::can_block(game, *b, *a)).collect()))
                .filter(|(_, a): &(ObjId, Vec<ObjId>)| !a.is_empty())
                .collect();
            let plan = |blocks: &[(ObjId, ObjId)]| {
                let mut p = Vec::new();
                for (b, a) in blocks {
                    p.push(Answer::Obj(*b));
                    // the engine asks which attacker only when there's a choice
                    if options.iter().find(|(x, _)| x == b).is_some_and(|(_, o)| o.len() > 1) {
                        p.push(Answer::Obj(*a));
                    }
                }
                p.push(Answer::Bool(true));
                p
            };
            add(plan(&[]));
            let combos: usize = options.iter().map(|(_, o)| o.len() + 1).product();
            if combos <= MAX_PLANS * 2 {
                // every assignment, double blocks included
                for n in 1..combos {
                    let mut k = n;
                    let mut blocks = Vec::new();
                    for (b, o) in &options {
                        let pick = k % (o.len() + 1);
                        k /= o.len() + 1;
                        if pick > 0 {
                            blocks.push((*b, o[pick - 1]));
                        }
                    }
                    add(plan(&blocks));
                }
            } else {
                // Large boards (e.g. token swarms): include greedy multi-block plans that stop lethal / trample damage
                // before falling back to single-blocker plans.
                let defender = game.obj(options[0].0).controller;
                let life = game.player(defender).life;
                let mut rem: HashMap<ObjId, i32> =
                    attackers.iter().map(|a| (*a, incoming_player_damage(game, *a))).collect();
                let mut sorted_opts = options.clone();
                sorted_opts.sort_by_key(|(b, _)| eval::permanent_score(game, *b));
                let mut anti_lethal = Vec::new();
                for (b, o) in &sorted_opts {
                    let total_rem: i32 = rem.values().sum();
                    if total_rem < life {
                        break;
                    }
                    let best = o
                        .iter()
                        .copied()
                        .filter_map(|a| {
                            let r = *rem.get(&a).unwrap_or(&0);
                            if r <= 0 {
                                return None;
                            }
                            let trample = game.has_ability(a, &AbilityDef::TrampleAbility);
                            let prevented = if !trample {
                                r
                            } else if game.has_ability(a, &AbilityDef::DeathtouchAbility) {
                                r.min(1)
                            } else {
                                r.min(game.power_toughness(*b).1.max(0))
                            };
                            (prevented > 0).then_some((a, prevented, r))
                        })
                        .max_by_key(|(_, prevented, r)| (*prevented, *r));
                    if let Some((a, prevented, _)) = best {
                        anti_lethal.push((*b, a));
                        *rem.entry(a).or_insert(0) -= prevented;
                    }
                }
                if !anti_lethal.is_empty() {
                    add(plan(&anti_lethal));
                }
                for (b, o) in &options {
                    for a in o {
                        add(plan(&[(*b, *a)]));
                    }
                }
            }
        }
        _ => return None,
    }
    Some(out)
}
