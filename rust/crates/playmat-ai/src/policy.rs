//! The quick answers: what a seat says when nothing is searched. Searches play their games out with these, for
//! both seats, so they must be cheap and never get stuck (the engine asks again after an answer it can't use, and
//! the search's answer budget ends any loop).

use crate::eval;
use playmat_engine::card::{AbilityDef, CardType, ManaCost};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::rules;
use playmat_engine::state::{Game, Obj, ObjId, Seat, Step, Zone};
use std::collections::HashMap;
use std::sync::Arc;

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
    /// optional Forge-inspired AI playbook guiding this seat's heuristics
    playbook: Option<Arc<crate::playbook::Playbook>>,
}

#[derive(Clone, PartialEq, Eq)]
struct Progress([u64; 9]);

/// A yes/no question the AI always declines, as Java's `Outcome.AIDontUseIt` makes `ComputerPlayer.chooseUse` say no:
/// replicate's "Pay N times Replicate {1} ?" (`ReplicateAbility`), which is asked again after each yes and never
/// checks the mana for the copies (game 5b6e1d86 on 2026-11-16: an AI that said yes was asked forever)
pub fn declines(message: &str) -> bool {
    message.starts_with("Pay ") && message.contains(" Replicate ")
}

/// Parse a leading "Pay <N> life ..." prompt (e.g. Phyrexian "Pay 2 life instead of {W}?" or life attack taxes).
fn life_payment_amount(message: &str) -> Option<i32> {
    let trimmed = message.trim_start();
    let rest = trimmed.strip_prefix("Pay ").or_else(|| trimmed.strip_prefix("pay "))?;
    let mut parts = rest.split_whitespace();
    let n: i32 = parts.next()?.parse().ok()?;
    let unit = parts.next()?;
    (n > 0 && unit.trim_end_matches(|c: char| !c.is_ascii_alphabetic()).eq_ignore_ascii_case("life")).then_some(n)
}

/// If `message` is a Phyrexian life-replacement prompt ("Pay 2 life instead of {W}?"), return the WUBRG index (0..5).
fn phyrexian_color_index(message: &str) -> Option<usize> {
    let rest = message.trim_start().strip_prefix("Pay 2 life instead of {")?;
    match rest.as_bytes().first()? {
        b'W' => Some(0),
        b'U' => Some(1),
        b'B' => Some(2),
        b'R' => Some(3),
        b'G' => Some(4),
        _ => None,
    }
}

fn can_pay_color_without_life(game: &Game, seat: Seat, color: usize) -> bool {
    if game.player(seat).mana_pool.colored[color] > 0 {
        return true;
    }
    game.battlefield.iter().any(|id| {
        let o = game.obj(*id);
        o.controller == seat
            && !o.tapped
            && o.def.abilities.iter().any(|a| a.mana().is_some_and(|m| m.colored[color] > 0))
    })
}

/// Whether the AI should decline a yes/no prompt in the current game state: replicate (always) or any lethal life
/// payment (such as paying 2 life at 2 life for Norn's Annex, #419).
pub fn declines_in_game(game: &Game, seat: Seat, message: &str) -> bool {
    if declines(message) {
        return true;
    }
    if let Some(life_cost) = life_payment_amount(message) {
        if game.player(seat).life <= life_cost {
            return true;
        }
    }
    false
}

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

    /// The quick answers of a `Config::playout` version guided by an optional Playbook
    pub fn with_playbook(v: u8, playbook: Option<Arc<crate::playbook::Playbook>>) -> Policy {
        Policy { v2: v >= 2, playbook, ..Policy::default() }
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
            Query::Attackers { creatures } => attack_with(game, seat, creatures, self.playbook.as_deref()),
            Query::Blockers { creatures } => self.block(game, seat, creatures),
            Query::BlockTarget { attackers } => self.block_target(game, attackers),
            Query::Cards { objs, required } => cards_with(game, seat, objs, *required, self.playbook.as_deref()),
            Query::MultiAmount { defaults, .. } => {
                Answer::Text(defaults.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(" "))
            }
            Query::ChooseAbility { source, abilities } if self.v2 => loyalty_ability(game, seat, *source, abilities)
                .or_else(|| {
                    // activated at priority, not paying for something: an ability that does more than make mana
                    let paying = game.unpaid.is_some() || game.mana_paid.is_some();
                    let def = &game.obj(*source).def;
                    (!paying)
                        .then(|| {
                            abilities
                                .iter()
                                .copied()
                                .find(|i| def.abilities.get(*i).is_some_and(|a| a.mana().is_none()))
                        })
                        .flatten()
                        .map(|i| Answer::Ability(*source, i))
                })
                .unwrap_or_else(|| choose_ability(game, seat, *source, abilities)),
            Query::ChooseAbility { source, abilities } => {
                let paying = game.unpaid.is_some() || game.mana_paid.is_some();
                let def = &game.obj(*source).def;
                (!paying)
                    .then(|| {
                        abilities.iter().copied().find(|i| def.abilities.get(*i).is_some_and(|a| a.mana().is_none()))
                    })
                    .flatten()
                    .map(|i| Answer::Ability(*source, i))
                    .unwrap_or_else(|| choose_ability(game, seat, *source, abilities))
            }
            Query::PickTrigger { .. } => Answer::Trigger(0),
            Query::YesNo { message } => {
                let decline = declines_in_game(game, seat, message)
                    || phyrexian_color_index(message)
                        .is_some_and(|color| can_pay_color_without_life(game, seat, color));
                Answer::Bool(!decline)
            }
            Query::ChooseMode { modes } => self.choose_mode(game, seat, modes),
            Query::Amount { min, max } => Answer::Int(x_amount(game, seat).clamp(*min, (*max).max(*min))),
            Query::Choice { message, choices } => choice(game, seat, message, choices),
        }
    }

    fn choose_mode(&self, game: &Game, seat: Seat, modes: &[usize]) -> Answer {
        if let (Some(pb), Some(top)) = (self.playbook.as_deref(), game.stack.last()) {
            let name = game.obj(top.card).def.name.as_str();
            if let Some(m) = pb.preferred_mode(name, game, seat, modes) {
                return Answer::Mode(m);
            }
        }
        modes.first().map(|m| Answer::Mode(*m)).unwrap_or(Answer::ModeDone)
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

    /// Blocks that kill the attacker or survive it, and chump/gang-blocks (including tramplers) when the attack is
    /// lethal (or below playbook threshold).
    fn block(&mut self, game: &Game, seat: Seat, creatures: &[ObjId]) -> Answer {
        let attackers = rules::attackers(game);
        let unblocked: Vec<ObjId> = attackers.iter().copied().filter(|a| !game.obj(*a).blocked).collect();
        let incoming: i32 = attackers.iter().map(|a| incoming_player_damage(game, *a)).sum();
        let life = game.player(seat).life;
        let chump_at = self.playbook.as_deref().map_or(0, |pb| pb.chump_block_life_threshold);
        let lethal = incoming >= life || (chump_at > 0 && life - incoming <= chump_at && incoming >= 4);
        let available: Vec<ObjId> = creatures
            .iter()
            .copied()
            .filter(|b| !self.tried_blockers.contains(b) && game.obj(*b).blocking.is_empty())
            .collect();

        let true_lethal = incoming >= life;
        for b in &available {
            let (bp, bt) = game.power_toughness(*b);
            let b_deathtouch = game.has_ability(*b, &AbilityDef::DeathtouchAbility);
            let protected = self.playbook.as_deref().is_some_and(|pb| pb.is_protected(&game.obj(*b).def.name));
            let max_prev = if true_lethal {
                attackers
                    .iter()
                    .copied()
                    .filter(|a| rules::can_block(game, *b, *a))
                    .map(|a| damage_prevented(game, *b, a))
                    .max()
                    .unwrap_or(0)
            } else {
                0
            };
            let good = unblocked.iter().copied().filter(|a| rules::can_block(game, *b, *a)).find(|a| {
                if true_lethal {
                    let prev = damage_prevented(game, *b, *a);
                    // Never waste a blocker on a small "good trade" while dying to a bigger unblocked attacker.
                    if incoming - prev >= life && prev < max_prev {
                        return false;
                    }
                }
                let (ap, at) = game.power_toughness(*a);
                let a_deathtouch = game.has_ability(*a, &AbilityDef::DeathtouchAbility);
                let survives = bt > ap && !(a_deathtouch && ap > 0);
                if protected && !survives {
                    return false;
                }
                let kills = bp >= at || (b_deathtouch && bp > 0);
                survives || (kills && eval::permanent_score(game, *a) >= eval::permanent_score(game, *b))
            });
            if let Some(a) = good {
                self.tried_blockers.push(*b);
                self.blocker = Some(a);
                return Answer::Obj(*b);
            }
        }

        if lethal {
            // Sac cheapest available blocker first (e.g. tokens before real creatures) to stop lethal / trample overflow
            let mut chumpers: Vec<ObjId> = available
                .iter()
                .copied()
                .filter(|b| {
                    let protected = self.playbook.as_deref().is_some_and(|pb| pb.is_protected(&game.obj(*b).def.name));
                    !protected || incoming >= life
                })
                .collect();
            chumpers.sort_by_key(|b| eval::permanent_score(game, *b));
            for b in chumpers {
                let target = attackers
                    .iter()
                    .copied()
                    .filter(|a| rules::can_block(game, b, *a) && damage_prevented(game, b, *a) > 0)
                    .max_by_key(|a| (damage_prevented(game, b, *a), incoming_player_damage(game, *a)));
                if let Some(a) = target {
                    self.tried_blockers.push(b);
                    self.blocker = Some(a);
                    return Answer::Obj(b);
                }
            }
        }

        Answer::Bool(true)
    }

    fn block_target(&mut self, game: &Game, attackers: &[ObjId]) -> Answer {
        match self.blocker.take() {
            Some(a) if attackers.contains(&a) => Answer::Obj(a),
            _ => attackers
                .iter()
                .max_by_key(|a| (incoming_player_damage(game, **a), game.power_toughness(**a).0))
                .map(|a| Answer::Obj(*a))
                .unwrap_or(Answer::Bool(false)),
        }
    }
}

/// Combat damage this attacker is still projected to deal to the defending player given currently declared blockers,
/// accounting for trample overflow, double strike, and attacker deathtouch.
pub(crate) fn incoming_player_damage(game: &Game, attacker: ObjId) -> i32 {
    let o = game.obj(attacker);
    if o.attacking_pw.is_some() {
        return 0;
    }
    let ap = game.power_toughness(attacker).0.max(0);
    let total = if game.has_ability(attacker, &AbilityDef::DoubleStrikeAbility) { ap * 2 } else { ap };
    if !o.blocked {
        return total;
    }
    if !game.has_ability(attacker, &AbilityDef::TrampleAbility) {
        return 0;
    }
    let deathtouch = game.has_ability(attacker, &AbilityDef::DeathtouchAbility);
    let absorbed: i32 =
        o.blocked_by.iter().map(|b| if deathtouch { 1 } else { game.power_toughness(*b).1.max(0) }).sum();
    (total - absorbed).max(0)
}

/// How much projected player damage adding `blocker` to `attacker` would prevent.
pub(crate) fn damage_prevented(game: &Game, blocker: ObjId, attacker: ObjId) -> i32 {
    let rem = incoming_player_damage(game, attacker);
    if rem <= 0 {
        return 0;
    }
    if !game.has_ability(attacker, &AbilityDef::TrampleAbility) {
        return rem;
    }
    let absorb = if game.has_ability(attacker, &AbilityDef::DeathtouchAbility) {
        1
    } else {
        game.power_toughness(blocker).1.max(0)
    };
    rem.min(absorb)
}

/// Attacks with creatures guided by combat heuristics and optional Playbook role protection + aggression.
fn attack_with(game: &Game, seat: Seat, creatures: &[ObjId], pb: Option<&crate::playbook::Playbook>) -> Answer {
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
    let mut atk_powers: Vec<i32> = creatures
        .iter()
        .chain(rules::attackers(game).iter())
        .map(|c| game.power_toughness(*c).0.max(0))
        .collect();
    let power: i32 = atk_powers.iter().sum();
    atk_powers.sort_unstable_by(|a, b| b.cmp(a));
    // Guaranteed unblocked damage even if opponent blocks the largest attackers (ignoring flying/evasion, conservative).
    let unblocked_min: i32 = atk_powers.iter().skip(blockers.len()).sum();
    let hand_pump: i32 = if atk_powers.len() > blockers.len() {
        let avail_mana = crate::playbook::available_mana(game, seat);
        game.player(seat)
            .hand
            .iter()
            .filter_map(|id| {
                let def = &game.obj(*id).def;
                let k = spell_kind(def);
                if !def.types.contains(&CardType::Instant) || !k.pump || def.mana_value() > avail_mana {
                    return None;
                }
                let colors_ok = def
                    .cost
                    .colored
                    .iter()
                    .enumerate()
                    .all(|(ci, &need)| need == 0 || can_pay_color_without_life(game, seat, ci));
                colors_ok.then_some(k.pump_power.unwrap_or(2))
            })
            .max()
            .unwrap_or(0)
    } else {
        0
    };
    let lethal = power >= game.player(opp).life + blockers.len() as i32 * 2
        || (unblocked_min > 0 && unblocked_min + hand_pump >= game.player(opp).life);
    if blockers.is_empty() && !creatures.is_empty() {
        // Even with no blockers, control/ramp/combo decks keep protected mana dorks back unless lethal or they deal >= 2
        if pb.is_some_and(|p| p.aggression < 0) && !lethal {
            for c in creatures {
                let (cp, _) = game.power_toughness(*c);
                if cp <= 0 {
                    continue;
                }
                if pb.is_some_and(|p| p.has_role(&game.obj(*c).def.name, "mana_dork")) && cp <= 1 {
                    continue;
                }
                return Answer::Obj(*c);
            }
            return Answer::Bool(true);
        }
        return Answer::Text("special".into());
    }
    if lethal && !creatures.is_empty() {
        return Answer::Text("special".into());
    }
    let aggro = pb.map_or(0, |p| p.aggression);
    for c in creatures {
        let (cp, ct) = game.power_toughness(*c);
        if cp <= 0 {
            continue;
        }
        let name = game.obj(*c).def.name.as_str();
        let protected = pb.is_some_and(|p| p.is_protected(name));
        let dies = blockers.iter().any(|b| rules::can_block(game, *b, *c) && game.power_toughness(*b).0 >= ct);
        if !dies {
            // Protected utility creatures (e.g. mana dorks) still don't swing into bad taps when defensive
            if protected && aggro < 0 && cp <= 1 {
                continue;
            }
            return Answer::Obj(*c);
        }
        // Aggressive playbooks happily trade non-protected attackers into equal or smaller blockers
        if !protected && aggro >= 40 {
            let trades_up_or_even = blockers
                .iter()
                .filter(|b| rules::can_block(game, **b, *c) && game.power_toughness(**b).0 >= ct)
                .all(|b| cp >= game.power_toughness(*b).1);
            if trades_up_or_even {
                return Answer::Obj(*c);
            }
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
            let animates = game
                .obj(**s)
                .def
                .abilities
                .iter()
                .enumerate()
                .any(|(i, _)| ability_kind(&game.obj(**s).def, i).animate);
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

/// Cards from the seat's own hand go least valuable first (discards), respecting Playbook discard roles (e.g.
/// discarding Reanimator targets first!); library/other cards respect tutor priority or go most valuable first.
fn cards_with(
    game: &Game,
    seat: Seat,
    objs: &[ObjId],
    required: bool,
    pb: Option<&crate::playbook::Playbook>,
) -> Answer {
    let own_hand = objs.iter().all(|o| game.obj(*o).zone == Zone::Hand && game.obj(*o).owner == seat);
    if !required && own_hand {
        return Answer::Bool(false);
    }
    if own_hand {
        if let Some(pb) = pb {
            if !pb.discard_roles_first.is_empty() {
                if let Some(target) =
                    objs.iter().find(|o| pb.has_any_role(&game.obj(**o).def.name, &pb.discard_roles_first))
                {
                    return Answer::Obj(*target);
                }
            }
            let pick = objs.iter().min_by_key(|o| {
                let name = game.obj(**o).def.name.as_str();
                let keep_penalty = if pb.has_any_role(name, &pb.never_discard_roles) { 10_000 } else { 0 };
                eval::hand_card_score(game, **o) + keep_penalty
            });
            return pick.map(|o| Answer::Obj(*o)).unwrap_or(Answer::Bool(false));
        }
        let pick = objs.iter().min_by_key(|o| eval::hand_card_score(game, **o));
        return pick.map(|o| Answer::Obj(*o)).unwrap_or(Answer::Bool(false));
    }
    // Tutor / library search priority from active spell's CardRule
    if let (Some(pb), Some(top)) = (pb, game.stack.last()) {
        if let Some(rule) = pb.rule(&game.obj(top.card).def.name) {
            for want in &rule.tutor_priority {
                if let Some(o) = objs.iter().find(|id| game.obj(**id).def.name == *want) {
                    return Answer::Obj(*o);
                }
            }
        }
    }
    let pick = objs.iter().max_by_key(|o| {
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
    });
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
    if let Some(name) = card_name(game, seat, message) {
        return Answer::Text(name);
    }
    Answer::Text(String::new())
}

/// "Choose a card name" (Pithing Needle, Phyrexian Revoker: the chosen name's activated abilities other than mana
/// abilities can't be activated): an opponent's card that fits the question, seen in a public zone, best one whose
/// abilities it could activate from where it is, then their lands, and never one of the seat's own. Nothing seen:
/// Island when a land fits, else any fitting card that isn't the seat's. None when the message isn't a card name
/// question.
fn card_name(game: &Game, seat: Seat, message: &str) -> Option<String> {
    game.objs.first()?.def.fits_name_question(message)?;
    let fits = |o: &Obj| o.def.fits_name_question(message) == Some(true);
    let mine: Vec<&str> =
        game.objs.iter().filter(|o| o.owner == seat && o.zone != Zone::Library).map(|o| o.def.name.as_str()).collect();
    let theirs = |o: &&Arc<Obj>| o.owner != seat && fits(o) && !mine.contains(&o.def.name.as_str());
    let mut scores: HashMap<&str, usize> = HashMap::new();
    for o in game.objs.iter().filter(theirs) {
        if !matches!(o.zone, Zone::Battlefield | Zone::Graveyard | Zone::Exile) {
            continue;
        }
        let usable = o.def.abilities.iter().filter(|a| a.activated().is_some()).any(|a| match o.zone {
            Zone::Battlefield => !a.from_hand() && !a.from_graveyard(),
            Zone::Graveyard => a.from_graveyard(),
            _ => false,
        });
        let any = o.def.abilities.iter().any(|a| a.activated().is_some());
        let land = o.def.is(CardType::Land) && o.zone == Zone::Battlefield;
        let score = if usable {
            100
        } else if any {
            10
        } else if land {
            1
        } else {
            0
        };
        if score > 0 {
            *scores.entry(o.def.name.as_str()).or_default() += score;
        }
    }
    let best = scores.into_iter().max_by_key(|(n, c)| (*c, std::cmp::Reverse(*n))).map(|(n, _)| n.to_string());
    let island = ["Card name", "Choose a card name", "Land card name", "Choose a land card name"].contains(&message);
    best.or_else(|| island.then(|| "Island".to_string()))
        .or_else(|| game.objs.iter().filter(theirs).map(|o| o.def.name.clone()).min())
        .or_else(|| Some(String::new()))
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
    /// power boost granted by a pump spell (e.g. +3 for Giant Growth)
    pub pump_power: Option<i32>,
    /// draws or looks for cards
    pub draw: bool,
    /// a land or permanent that becomes a creature (Mutavault)
    pub animate: bool,
    /// destroys or damages every creature
    pub sweeper: bool,
}

/// The kind of a card's spell (memoized per thread to avoid repeated string/Debug allocations in hot loops)
pub fn spell_kind(def: &playmat_engine::card::CardDef) -> Kind {
    thread_local! {
        static CACHE: std::cell::RefCell<HashMap<(usize, u32), Kind>> = std::cell::RefCell::new(HashMap::with_capacity(512));
    }
    let key = (def as *const playmat_engine::card::CardDef as usize, crate::obs::card_id(&def.name));
    CACHE.with(|c| {
        if let Some(&k) = c.borrow().get(&key) {
            return k;
        }
        let k = kind_of(&def.spell.rule.to_lowercase(), &format!("{:?}", def.spell.effects));
        let mut map = c.borrow_mut();
        if map.len() >= 4096 {
            map.clear();
        }
        map.insert(key, k);
        k
    })
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
    let pump = (rule.contains("target creature gets +")
        || rule.contains("target attacking creature gets +")
        || effects.contains("BoostTargetEffect"))
        && !rule.contains("gets -");
    let pump_power = if pump {
        rule.find("gets +").and_then(|at| {
            let rest = &rule[at + 6..];
            let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            n.parse::<i32>().ok()
        })
    } else {
        None
    };
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
        pump,
        pump_power,
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
        let castable: Vec<ObjId> = own
            .iter()
            .copied()
            .filter(|id| matches!(game.obj(*id).zone, Zone::Hand | Zone::Graveyard | Zone::Exile))
            .collect();
        let kind = |id: ObjId| spell_kind(&game.obj(id).def);
        let is = |id: ObjId, t: CardType| game.obj(id).def.types.contains(&t);
        let instant_speed = |id: ObjId| {
            is(id, CardType::Instant)
                || game
                    .obj(id)
                    .def
                    .abilities
                    .iter()
                    .any(|a| matches!(a, playmat_engine::card::AbilityDef::FlashAbility))
        };
        let creatures = |s: Seat| -> Vec<ObjId> {
            game.battlefield
                .iter()
                .copied()
                .filter(|id| game.obj(*id).controller == s && game.has_type(*id, CardType::Creature))
                .collect()
        };

        let pb = self.playbook.as_deref();
        let min_counter_mv = pb.map_or(2, |p| p.min_counter_mv);

        // the stack: answer an opponent's spell, or let it resolve
        if let Some(top) = game.stack.last() {
            if top.controller == opp && top.ability.is_none() {
                let spell = &game.obj(top.card).def;
                let worth = spell.mana_value() >= min_counter_mv
                    || spell.types.contains(&CardType::Planeswalker)
                    || spell_kind(spell).removal
                    || spell_kind(spell).sweeper;
                if worth {
                    if let Some(c) = castable.iter().copied().find(|c| {
                        let name = game.obj(*c).def.name.as_str();
                        kind(*c).counter || pb.is_some_and(|p| p.has_role(name, "counter"))
                    }) {
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
                // instant-speed cards scripted for opponent's attackers (e.g. The Wandering Emperor) or removal
                if let Some(c) = castable.iter().copied().find(|c| {
                    let name = game.obj(*c).def.name.as_str();
                    instant_speed(*c)
                        && (kind(*c).removal
                            || pb.is_some_and(|p| {
                                p.rule(name).is_some_and(|r| {
                                    r.hold_for_step == Some(crate::playbook::HoldStep::CombatAttackers)
                                }) && !p.should_hold_card(name, game, seat)
                            }))
                }) {
                    return Answer::Obj(c);
                }
            }
            if step == Some(Step::DeclareBlockers) {
                let fighting = attackers.iter().any(|a| {
                    let o = game.obj(*a);
                    (mine && o.blocked) || (!mine && !o.blocked_by.is_empty())
                });
                let has_unblocked = mine && attackers.iter().any(|a| !game.obj(*a).blocked);
                let unblocked_dmg: i32 = if has_unblocked {
                    attackers.iter().map(|a| incoming_player_damage(game, *a)).sum()
                } else {
                    0
                };
                let opp_life = game.player(opp).life;
                if let Some(c) = castable.iter().copied().find(|c| {
                    if !instant_speed(*c) {
                        return false;
                    }
                    let k = kind(*c);
                    if !k.pump {
                        return false;
                    }
                    if fighting {
                        return true;
                    }
                    has_unblocked && unblocked_dmg + k.pump_power.unwrap_or(2) >= opp_life
                }) {
                    return Answer::Obj(c);
                }
            }
            return Answer::Bool(false);
        }

        if !mine {
            // the opponent's end step: use the mana left (including Playbook eot_instant / Collected Company / draw)
            if step == Some(Step::EndTurn) {
                let opp_creatures = !creatures(opp).is_empty();
                let pick = castable
                    .iter()
                    .copied()
                    .filter(|c| instant_speed(*c) && !is(*c, CardType::Land))
                    .filter(|c| {
                        let name = game.obj(*c).def.name.as_str();
                        if pb.is_some_and(|p| p.should_hold_card(name, game, seat)) {
                            return false;
                        }
                        let k = kind(*c);
                        let eot_tagged = pb.is_some_and(|p| {
                            p.has_role(name, "eot_instant")
                                || p.rule(name)
                                    .is_some_and(|r| r.hold_for_step == Some(crate::playbook::HoldStep::OppEndStep))
                        });
                        eot_tagged || (k.removal && opp_creatures) || k.draw || is(*c, CardType::Creature)
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
        if let Some(land) =
            castable.iter().copied().find(|id| is(*id, CardType::Land) && game.obj(*id).zone == Zone::Hand)
        {
            return Answer::Obj(land);
        }
        // burn to the face when lethal or opponent is within the Playbook's burn-face threshold
        let opp_life = game.player(opp).life;
        let burn_threshold = pb.map_or(0, |p| p.burn_face_threshold_life);
        let burn: Vec<ObjId> =
            castable.iter().copied().filter(|c| kind(*c).face && kind(*c).damage.is_some()).collect();
        let total: i32 = burn.iter().map(|c| kind(*c).damage.unwrap_or(0)).sum();
        if total >= opp_life || (burn_threshold > 0 && opp_life <= burn_threshold && !burn.is_empty()) {
            if let Some(c) = burn.iter().copied().max_by_key(|c| kind(*c).damage) {
                return Answer::Obj(c);
            }
        }
        let my_count = creatures(seat).len();
        let opp_count = creatures(opp).len();
        let my_life = game.player(seat).life;
        let avail_mana = crate::playbook::available_mana(game, seat);
        let reserve = pb.map_or(0, |p| p.reserve_mana(game, seat, &castable));
        let precombat = step == Some(Step::PrecombatMain);
        let has_attackers = precombat
            && creatures(seat).iter().any(|id| {
                let o = game.obj(*id);
                !o.tapped && (!o.sick || game.has_ability(*id, &playmat_engine::card::AbilityDef::HasteAbility))
            });
        let sweeper_net = pb.map_or(1, |p| p.sweeper_min_net_creatures);
        let sweeper_life = pb.map_or(0, |p| p.sweeper_emergency_life);

        let spell = castable
            .iter()
            .copied()
            .filter(|c| {
                let o = game.obj(*c);
                let name = o.def.name.as_str();
                let k = kind(*c);
                if is(*c, CardType::Land) || k.counter || pb.is_some_and(|p| p.has_role(name, "counter")) {
                    return false;
                }
                if pb.is_some_and(|p| p.should_hold_card(name, game, seat)) {
                    return false;
                }
                // tricks wait for combat
                if k.pump && !is(*c, CardType::Creature) {
                    return false;
                }
                // a sweeper when net opponent creatures meet threshold, or emergency life
                let is_sweeper = k.sweeper || pb.is_some_and(|p| p.has_role(name, "sweeper"));
                if is_sweeper {
                    let net = opp_count as i32 - my_count as i32;
                    let emergency = sweeper_life > 0 && my_life <= sweeper_life && opp_count > 0;
                    if net < sweeper_net && !emergency {
                        return false;
                    }
                }
                // removal needs something to remove
                if k.removal && !k.face && opp_count == 0 && !is(*c, CardType::Creature) {
                    return false;
                }
                // Playbook instant-speed mana discipline: don't tap out of reserved counter/EOT mana
                if reserve > 0 && !is_sweeper {
                    let mv = o.def.mana_value();
                    let is_eot = pb.is_some_and(|p| p.has_role(name, "eot_instant"));
                    if is_eot || avail_mana.saturating_sub(mv) < reserve {
                        return false;
                    }
                }
                // Playbook main-2 creature discipline: attack first before deploying non-haste, non-lord creatures
                if precombat && has_attackers && is(*c, CardType::Creature) {
                    let haste =
                        o.def.abilities.iter().any(|a| matches!(a, playmat_engine::card::AbilityDef::HasteAbility));
                    let prefer_m1 = pb.is_some_and(|p| {
                        p.has_role(name, "lord")
                            || p.has_role(name, "haste")
                            || p.rule(name).is_some_and(|r| r.prefer_before_combat)
                    });
                    let defer_m2 =
                        pb.is_some_and(|p| p.cast_creatures_main2 || p.rule(name).is_some_and(|r| r.prefer_main2));
                    if defer_m2 && !haste && !prefer_m1 {
                        return false;
                    }
                }
                true
            })
            .max_by_key(|c| {
                let o = game.obj(*c);
                let rank = pb.map_or(usize::MAX, |p| p.play_rank(&o.def.name));
                (std::cmp::Reverse(rank), o.def.mana_value())
            });
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
    fn target_v2(
        &mut self,
        game: &Game,
        seat: Seat,
        players: &[Seat],
        objs: &[ObjId],
        harmful: bool,
    ) -> Option<Answer> {
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
        // help: lethal pump on an unblocked attacker first, else its own creature in a fight
        if k.pump && matches!(game.step, Some(Step::DeclareBlockers)) && game.active == Some(seat) {
            let attackers = rules::attackers(game);
            let unblocked_dmg: i32 = attackers.iter().map(|a| incoming_player_damage(game, *a)).sum();
            let opp_life = game.player(opp).life;
            let boost = k.pump_power.unwrap_or(2);
            let lethal_unblocked = objs
                .iter()
                .copied()
                .filter(|o| {
                    let x = game.obj(*o);
                    x.controller == seat && x.attacking.is_some() && !x.blocked && x.attacking_pw.is_none()
                })
                .filter(|o| {
                    let mult = if game.has_ability(*o, &AbilityDef::DoubleStrikeAbility) { 2 } else { 1 };
                    unblocked_dmg + boost * mult >= opp_life
                })
                .max_by_key(|o| (game.has_ability(*o, &AbilityDef::DoubleStrikeAbility), eval::permanent_score(game, *o)));
            if let Some(o) = lethal_unblocked {
                return Some(Answer::Obj(o));
            }
        }
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::duel::{new_duel, Deck};

    /// A card name goes to the opponent's card with an activated ability, never one of the seat's own
    #[test]
    fn names_the_opponents_activated_card() {
        let db = playmat_engine::cards();
        let mine = Deck::parse("a", "20 Plains\n1 Aether Vial");
        let theirs = Deck::parse("b", "20 Plains\n1 Aether Vial\n1 Arid Mesa");
        let mut game = new_duel(&db, [&mine, &theirs], 1, 0).unwrap();
        let on = |game: &Game, seat: Seat, name: &str| {
            game.objs.iter().find(|o| o.owner == seat && o.def.name == name && o.zone != Zone::Battlefield).unwrap().id
        };
        let id = on(&game, 0, "Aether Vial");
        game.move_to(id, Zone::Library);
        for name in ["Aether Vial", "Arid Mesa", "Plains"] {
            let id = on(&game, 1, name);
            game.move_to(id, Zone::Battlefield);
        }
        assert_eq!(card_name(&game, 0, "Card name").unwrap(), "Aether Vial");
        assert_eq!(card_name(&game, 0, "Land card name").unwrap(), "Arid Mesa");
        assert_eq!(card_name(&game, 0, "Choose a nonland card name").unwrap(), "Aether Vial");
        // the seat's own Aether Vial, in its hand, isn't named
        let id = on(&game, 0, "Aether Vial");
        game.move_to(id, Zone::Hand);
        assert_eq!(card_name(&game, 0, "Card name").unwrap(), "Arid Mesa");
        // nothing of theirs seen: Island
        assert_eq!(card_name(&game, 1, "Card name").unwrap(), "Island");
        // not a name question: none
        assert_eq!(card_name(&game, 0, "Choose a color"), None);
    }

    /// Replicate's question is declined, by the quick answer and by the search (`Outcome.AIDontUseIt`): the engine
    /// asks again after every yes, so an AI that says yes is asked forever (game 5b6e1d86 on 2026-11-16)
    #[test]
    fn the_ai_declines_to_replicate() {
        use playmat_engine::decide::Decider;
        let db = playmat_engine::cards();
        let deck = Deck::parse("d", "20 Island\n4 Consign to Memory");
        let game = new_duel(&db, [&deck, &deck], 1, 0).unwrap();
        let q = Query::YesNo { message: "Pay 1 time Replicate {1} ?".into() };
        assert_eq!(Policy::default().answer(&game, 0, &q), Answer::Bool(false));
        let mut table = crate::search::Table::new(vec![Some(crate::search::Config::default()), None]);
        assert_eq!(table.answer(&game, 0, &q), Answer::Bool(false));
        // other questions still get a yes
        let other = Query::YesNo { message: "Pay Kicker {2} ?".into() };
        assert_eq!(Policy::default().answer(&game, 0, &other), Answer::Bool(true));
    }

    /// The AI never pays lethal life for Phyrexian costs like Norn's Annex and prefers colored mana when available (#419)
    #[test]
    fn the_ai_does_not_pay_lethal_phyrexian_life_and_prefers_mana() {
        use playmat_engine::decide::Decider;
        let db = playmat_engine::cards();
        let deck = Deck::parse("d", "20 Plains");
        let mut game = new_duel(&db, [&deck, &deck], 1, 0).unwrap();
        let q = Query::YesNo { message: "Pay 2 life instead of {W}?".into() };

        // at 2 life, paying 2 life is lethal: both policy and search decline
        game.player_mut(0).life = 2;
        assert_eq!(Policy::default().answer(&game, 0, &q), Answer::Bool(false));
        let mut table = crate::search::Table::new(vec![Some(crate::search::Config::default()), None]);
        assert_eq!(table.answer(&game, 0, &q), Answer::Bool(false));

        // at 4 life with no white mana available, policy can pay 2 life
        game.player_mut(0).life = 4;
        assert_eq!(Policy::default().answer(&game, 0, &q), Answer::Bool(true));

        // with an untapped Plains on the battlefield, policy prefers paying {W} over 2 life
        let plains = game.objs.iter().find(|o| o.owner == 0 && o.def.name == "Plains").unwrap().id;
        game.move_to(plains, Zone::Battlefield);
        assert_eq!(Policy::default().answer(&game, 0, &q), Answer::Bool(false));
    }
}
