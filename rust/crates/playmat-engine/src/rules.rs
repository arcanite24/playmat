//! The rules: turns and steps, priority, playing lands and casting spells, paying mana, resolving, combat,
//! state-based actions. The order of everything a player sees follows Java's `GameImpl.playPriority` and
//! `HumanPlayer` with the client's default settings, so a shadow game replays answer for answer.

use crate::card::{
    AbilityDef, CardType, Color, Condition, CostDef, Duration, EffectDef, Mana, ManaCost, TargetDef, Trigger,
};
use crate::decide::{Answer, Decider, Query};
use crate::state::{Boost, Delayed, Game, Grant, ObjId, Seat, StackItem, Step, Target, Zone};
use std::sync::Arc;

const MAX_HAND: usize = 7;

/// Plays from the first priority of the game (as `ShadowGame::start_game` leaves it) until a player loses, the decider
/// stops, or turn `stop_on_turn` would begin (Java's `GameOptions.stopOnTurn`; 0 plays on).
pub fn play_from_priority(game: &mut Game, d: &mut dyn Decider, stop_on_turn: u32) {
    let mut first = true;
    while !game.over && !d.stopped() {
        let step = match game.step {
            Some(s) => s,
            None => return,
        };
        if !first {
            begin_step(game, d, step);
        }
        first = false;
        if game.over || d.stopped() {
            break;
        }
        // untap has no priority; cleanup only when 514.3a applies (nothing in the engine yet makes it)
        if step != Step::Untap && step != Step::Cleanup {
            priority_round(game, d);
        }
        if game.over || d.stopped() {
            break;
        }
        end_step(game, step);
        match next_step(game, step) {
            Some(s) => game.step = Some(s),
            None => {
                if stop_on_turn != 0 && game.turn + 1 >= stop_on_turn {
                    return;
                }
                game.turn += 1;
                game.active = Some(game.opponent(game.active.unwrap_or(0)));
                game.step = Some(Step::Untap);
            }
        }
    }
    if game.over {
        d.traced(game);
    }
}

fn next_step(game: &Game, step: Step) -> Option<Step> {
    Some(match step {
        Step::Untap => Step::Upkeep,
        Step::Upkeep if game.turn == 1 => Step::PrecombatMain,
        Step::Upkeep => Step::Draw,
        Step::Draw => Step::PrecombatMain,
        Step::PrecombatMain => Step::BeginCombat,
        Step::BeginCombat => Step::DeclareAttackers,
        Step::DeclareAttackers if !attackers(game).is_empty() => Step::DeclareBlockers,
        Step::DeclareAttackers => Step::EndCombat,
        // the damage steps are skipped when no attackers are left (`CombatDamageStep.skipStep`)
        Step::DeclareBlockers | Step::FirstCombatDamage if attackers(game).is_empty() => Step::EndCombat,
        Step::DeclareBlockers if first_strike_in_combat(game) => Step::FirstCombatDamage,
        Step::DeclareBlockers | Step::FirstCombatDamage => Step::CombatDamage,
        Step::CombatDamage => Step::EndCombat,
        Step::EndCombat => Step::PostcombatMain,
        Step::PostcombatMain => Step::EndTurn,
        Step::EndTurn => Step::Cleanup,
        Step::Cleanup => return None,
    })
}

/// The turn-based actions at the start of a step.
fn begin_step(game: &mut Game, d: &mut dyn Decider, step: Step) {
    let active = game.active.unwrap_or(0);
    match step {
        Step::Untap => {
            for s in 0..game.seats() {
                let p = game.player_mut(s);
                p.lands_played = 0;
                p.attacked = false;
            }
            for id in Arc::clone(&game.battlefield).iter().copied() {
                if game.obj(id).controller == active {
                    let o = game.obj_mut(id);
                    o.tapped = false;
                    o.sick = false;
                }
            }
        }
        Step::Draw => draw(game, active, 1),
        Step::DeclareAttackers => {
            declare_attackers(game, d, active);
            if !attackers(game).is_empty() {
                game.player_mut(active).attacked = true;
            }
            // "whenever this attacks" triggers, in the order the attackers are listed
            for a in attackers(game) {
                let fired = game.triggers([a], |_, _, _, t| matches!(t, Trigger::Attacks | Trigger::EntersOrAttacks));
                game.pending.extend(fired);
            }
        }
        Step::DeclareBlockers => declare_blockers(game, d, game.opponent(active)),
        // "at end of combat" delayed triggers
        Step::EndCombat => {
            let delayed = std::mem::take(&mut game.at_end_of_combat);
            game.pending.extend(delayed);
        }
        // "at the beginning of <step>" triggers
        Step::Upkeep | Step::BeginCombat | Step::EndTurn => {
            let name = match step {
                Step::Upkeep => "upkeep",
                Step::BeginCombat => "combat",
                _ => "end",
            };
            let sources: Vec<ObjId> = game.battlefield.iter().copied().collect();
            let fired = game.triggers(sources, |_, _, you, t| {
                matches!(t, Trigger::StepBegins { step, who } if step == name && Game::whose(who, you, active))
            });
            game.pending.extend(fired);
        }
        Step::FirstCombatDamage => {
            game.first_strike_step = true;
            combat_damage(game, d, true)
        }
        Step::CombatDamage => combat_damage(game, d, false),
        Step::Cleanup => cleanup(game, d, active),
        _ => {}
    }
}

fn end_step(game: &mut Game, step: Step) {
    for s in 0..game.seats() {
        game.player_mut(s).mana_pool = Mana::default();
    }
    if step == Step::EndCombat {
        game.first_strike_step = false;
        for id in Arc::clone(&game.battlefield).iter().copied() {
            let o = game.obj(id);
            if o.attacking.is_some() || !o.blocking.is_empty() || o.blocked {
                let o = game.obj_mut(id);
                o.attacking = None;
                o.blocking.clear();
                o.blocked = false;
                o.blocked_by.clear();
            }
        }
    }
}

pub fn draw(game: &mut Game, seat: Seat, n: usize) {
    for _ in 0..n {
        match game.player(seat).library.first().copied() {
            Some(id) => game.move_to(id, Zone::Hand),
            None => game.player_mut(seat).drew_from_empty = true,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------- priority

/// A human seat with the default settings stops in these steps when the stack is empty; elsewhere it passes unasked.
fn stops_in(step: Step) -> bool {
    matches!(
        step,
        Step::PrecombatMain
            | Step::DeclareAttackers
            | Step::DeclareBlockers
            | Step::FirstCombatDamage
            | Step::CombatDamage
            | Step::PostcombatMain
    )
}

/// `GameImpl.playPriority`: each player in turn order from the active one until all pass in a row; then the top of
/// the stack resolves and priority starts again with the active player, or the step ends.
fn priority_round(game: &mut Game, d: &mut dyn Decider) {
    let active = game.active.unwrap_or(0);
    loop {
        for s in 0..game.seats() {
            game.player_mut(s).passed = false;
        }
        let mut seat = active;
        loop {
            game.priority = Some(seat);
            while !game.player(seat).passed {
                check_state_and_triggered(game, d);
                if game.over || d.stopped() {
                    return;
                }
                d.traced(game);
                if d.stopped() {
                    return;
                }
                priority(game, d, seat);
                if d.stopped() {
                    return;
                }
            }
            if game.players.iter().all(|p| p.passed) {
                break;
            }
            seat = game.opponent(seat);
        }
        if game.stack.is_empty() {
            return;
        }
        resolve_top(game, d);
        d.traced(game);
        if d.stopped() {
            return;
        }
    }
}

/// Asks a seat a question. Like `HumanPlayer.prepareForResponse`, asking makes that seat the priority player.
fn ask(game: &mut Game, d: &mut dyn Decider, seat: Seat, q: &Query) -> Answer {
    game.priority = Some(seat);
    d.answer(game, seat, q)
}

fn pass(game: &mut Game, seat: Seat) {
    game.player_mut(seat).passed = true;
}

/// `HumanPlayer.passWithManaPoolCheck`: passing with mana in the pool asks first. False: the player didn't pass.
fn pass_with_mana_pool_check(game: &mut Game, d: &mut dyn Decider, seat: Seat) -> bool {
    if game.stack.is_empty() && game.player(seat).mana_pool.total() > 0 {
        let q =
            Query::YesNo { message: "You still have mana in your mana pool and it will be lost. Pass anyway?".into() };
        loop {
            match ask(game, d, seat, &q) {
                Answer::Bool(true) => break,
                Answer::Bool(false) | Answer::Stop => return false,
                _ => {}
            }
        }
    }
    pass(game, seat);
    true
}

/// `HumanPlayer.priority`: one call; the player passes, plays something (and keeps priority), or is asked again.
fn priority(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
    let step = game.step.unwrap_or(Step::Upkeep);
    if game.stack.is_empty() && !stops_in(step) && pass_with_mana_pool_check(game, d, seat) {
        return;
    }
    loop {
        let playable = playable(game, seat);
        let answer = ask(game, d, seat, &Query::Priority { playable: playable.clone() });
        match answer {
            Answer::Stop => return,
            Answer::Bool(_) | Answer::Int(_) => {
                if pass_with_mana_pool_check(game, d, seat) || d.stopped() {
                    return;
                }
            }
            Answer::Obj(id) => {
                if playable.contains(&id) {
                    play(game, d, seat, id);
                }
                // played (priority again) or not playable (asked again): both come back to the priority loop
                return;
            }
            _ => return,
        }
    }
}

/// What `getPlayable` offers: lands to play, spells to cast that the player can pay for.
pub fn playable(game: &Game, seat: Seat) -> Vec<ObjId> {
    let mut out = Vec::new();
    let p = game.player(seat);
    for id in &p.hand {
        let o = game.obj(*id);
        if o.is(CardType::Land) {
            if can_play_land(game, seat) {
                out.push(*id);
            }
        } else if can_cast_timing(game, seat, *id) && can_pay(game, seat, &o.def.cost) && has_targets(game, seat, *id) {
            out.push(*id);
        }
    }
    for id in game.battlefield.iter() {
        if !activatable(game, seat, *id).is_empty() {
            out.push(*id);
        }
    }
    out
}

/// A permanent's activated abilities (not mana abilities) its controller can activate now, by index.
fn activatable(game: &Game, seat: Seat, id: ObjId) -> Vec<usize> {
    let o = game.obj(id);
    if o.controller != seat {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, a) in o.def.abilities.iter().enumerate() {
        let Some((_, _, targets, costs, sorcery)) = a.activated() else { continue };
        if sorcery && !sorcery_speed(game, seat) {
            continue;
        }
        let payable = costs.iter().all(|c| match c {
            CostDef::ManaCost { cost } => ManaCost::parse(cost).map(|m| can_pay(game, seat, &m)).unwrap_or(false),
            CostDef::TapSourceCost => {
                let sick = o.is(CardType::Creature) && o.sick && !o.has(&AbilityDef::HasteAbility);
                !(o.tapped || sick)
            }
            CostDef::SacrificeSourceCost => true,
        });
        let targeted = targets.iter().all(|t| target_possible(game, seat, t));
        if payable && targeted {
            out.push(i);
        }
    }
    out
}

fn ability_mana_cost(costs: &[CostDef]) -> ManaCost {
    costs
        .iter()
        .filter_map(|c| match c {
            CostDef::ManaCost { cost } => ManaCost::parse(cost).ok(),
            _ => None,
        })
        .fold(ManaCost::default(), |a, b| a.plus(&b))
}

fn sorcery_speed(game: &Game, seat: Seat) -> bool {
    game.active == Some(seat) && game.step.map(|s| s.is_main()).unwrap_or(false) && game.stack.is_empty()
}

fn can_play_land(game: &Game, seat: Seat) -> bool {
    sorcery_speed(game, seat) && game.player(seat).lands_played < 1
}

fn can_cast_timing(game: &Game, seat: Seat, id: ObjId) -> bool {
    let o = game.obj(id);
    o.is(CardType::Instant) || o.has(&AbilityDef::FlashAbility) || sorcery_speed(game, seat)
}

/// Whether the spell has its targets: for a modal spell, whether some mode does.
fn has_targets(game: &Game, seat: Seat, id: ObjId) -> bool {
    let spell = &game.obj(id).def.spell;
    let all = |targets: &[TargetDef]| targets.iter().all(|t| target_possible(game, seat, t));
    if spell.modes.is_empty() {
        all(&spell.targets)
    } else {
        spell.modes.iter().any(|m| all(&m.targets))
    }
}

/// Untapped permanents that can make mana now, each with its mana abilities (index, mana), in ability order.
fn mana_sources(game: &Game, seat: Seat) -> Vec<(ObjId, Vec<(usize, Mana)>)> {
    let mut out = Vec::new();
    for id in game.battlefield.iter() {
        let o = game.obj(*id);
        if o.controller != seat || o.tapped {
            continue;
        }
        if o.is(CardType::Creature) && o.sick && !o.has(&AbilityDef::HasteAbility) {
            continue;
        }
        let abilities: Vec<(usize, Mana)> =
            o.def.abilities.iter().enumerate().filter_map(|(i, a)| a.mana().map(|m| (i, m))).collect();
        if !abilities.is_empty() {
            out.push((*id, abilities));
        }
    }
    out
}

/// Whether the pool and untapped sources (one mana each, any of its abilities) can pay a cost.
fn can_pay(game: &Game, seat: Seat, cost: &ManaCost) -> bool {
    let mut options: Vec<Vec<Mana>> =
        mana_sources(game, seat).into_iter().map(|(_, a)| a.into_iter().map(|(_, m)| m).collect()).collect();
    let pool = game.player(seat).mana_pool;
    for c in Color::ALL {
        for _ in 0..pool.colored[c.index()] {
            options.push(vec![Mana::of(c, 1)]);
        }
    }
    for _ in 0..pool.colorless {
        options.push(vec![Mana { colorless: 1, ..Mana::default() }]);
    }
    // the costs' symbols, colored and colorless first; each needs a different source
    let mut needs: Vec<usize> = Vec::new();
    for c in Color::ALL {
        needs.extend(std::iter::repeat_n(c.index(), cost.colored[c.index()] as usize));
    }
    needs.extend(std::iter::repeat_n(5, cost.colorless as usize));
    let mut used = vec![false; options.len()];
    fn fits(m: &Mana, k: usize) -> bool {
        if k < 5 {
            m.colored[k] > 0
        } else {
            m.colorless > 0
        }
    }
    fn assign(needs: &[usize], options: &[Vec<Mana>], used: &mut [bool]) -> bool {
        let Some((&k, rest)) = needs.split_first() else { return true };
        for i in 0..options.len() {
            if !used[i] && options[i].iter().any(|m| fits(m, k)) {
                used[i] = true;
                if assign(rest, options, used) {
                    return true;
                }
                used[i] = false;
            }
        }
        false
    }
    if !assign(&needs, &options, &mut used) {
        return false;
    }
    used.iter().filter(|u| !**u).count() as u32 >= cost.generic
}

/// `ManaUtil.tryToAutoPay` for basic mana abilities: the one ability whose color is still owed; with none owed and
/// generic left, the first; otherwise all of them (the player picks).
fn auto_pay_abilities(abilities: &[(usize, Mana)], unpaid: &ManaCost) -> Vec<(usize, Mana)> {
    let owed: Vec<&(usize, Mana)> = abilities
        .iter()
        .filter(|(_, m)| Color::ALL.iter().any(|c| m.colored[c.index()] > 0 && unpaid.colored[c.index()] > 0))
        .collect();
    let colors: std::collections::BTreeSet<usize> = owed
        .iter()
        .flat_map(|(_, m)| Color::ALL.iter().filter(|c| m.colored[c.index()] > 0).map(|c| c.index()))
        .collect();
    match colors.len() {
        0 if unpaid.generic > 0 => abilities[..1].to_vec(),
        1 => vec![**owed.last().unwrap()],
        _ => abilities.to_vec(),
    }
}

fn play(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId) {
    if game.obj(id).zone == Zone::Battlefield {
        let abilities = activatable(game, seat, id);
        // `HumanPlayer.activateAbility`: one ability with mana in its cost activates directly; otherwise the picker
        let direct = abilities.len() == 1 && {
            let (_, _, _, costs, _) = game.obj(id).def.abilities[abilities[0]].activated().unwrap();
            ability_mana_cost(costs).mana_value() > 0
        };
        let index = if direct {
            abilities[0]
        } else {
            match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: abilities.clone() }) {
                Answer::Ability(o, i) if o == id && abilities.contains(&i) => i,
                _ => return,
            }
        };
        let before = game.clone();
        if !activate(game, d, seat, id, index) {
            *game = before;
        }
        return;
    }
    if game.obj(id).is(CardType::Land) {
        enter_battlefield(game, d, id, seat, false);
        game.player_mut(seat).lands_played += 1;
        return;
    }
    let before = game.clone();
    if !cast(game, d, seat, id) {
        // `PlayerImpl.cast` restores the game as it was before the cast
        *game = before;
    }
}

/// Casts a spell from hand: onto the stack, targets, then mana. False: cancelled.
fn cast(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId) -> bool {
    game.remove_from_zone(id);
    {
        let o = game.obj_mut(id);
        o.zone = Zone::Stack;
        o.zcc += 1;
    }
    Arc::make_mut(&mut game.stack).push(StackItem::spell(id, seat));
    let def = game.obj(id).def.clone();
    let mut cost = def.cost.clone();
    // kicker: `KickerAbility.addOptionalAdditionalCosts` asks before targets (its `canPay` holds for any mana cost)
    for a in &def.abilities {
        if let AbilityDef::KickerAbility { cost: kicker, .. } = a {
            let q = Query::YesNo { message: format!("Pay Kicker {} ?", kicker) };
            let pay = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pay {
                let Ok(k) = ManaCost::parse(kicker) else { return false };
                cost = cost.plus(&k);
                if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                    top.kicked = true;
                }
            }
        }
    }
    // `HumanPlayer.chooseMode`: one question for "choose one"; any answer but a mode (Cancel too) cancels the cast
    let mut mode = 0;
    if def.spell.modes.len() > 1 {
        let available: Vec<usize> = (0..def.spell.modes.len())
            .filter(|k| def.spell.modes[*k].targets.iter().all(|t| target_possible(game, seat, t)))
            .collect();
        mode = match ask(game, d, seat, &Query::ChooseMode { modes: available.clone() }) {
            Answer::Mode(k) if available.contains(&k) => k,
            _ => return false,
        };
        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
            top.mode = mode;
        }
    }
    if !choose_targets(game, d, seat, def.spell.mode(mode).1, Some(id), &def.spell.rule, true) {
        return false;
    }
    for s in 0..game.seats() {
        game.player_mut(s).passed = false;
    }
    if !pay_mana(game, d, seat, &cost) {
        return false;
    }
    // SPELL_CAST
    let spell = id;
    let sources: Vec<ObjId> = game.battlefield.iter().copied().collect();
    let fired = game.triggers(sources, |g, src, you, t| {
        matches!(t, Trigger::SpellCast { filter, who } if Game::whose(who, you, seat) && g.filter_matches(filter, you, Some(src), spell))
    });
    game.pending.extend(fired);
    true
}

/// Chooses the top stack object's targets, in order. False: cancelled (or, for a trigger, no legal targets).
fn choose_targets(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    defs: &[TargetDef],
    source: Option<ObjId>,
    rule: &str,
    cancellable: bool,
) -> bool {
    let mut targets = Vec::new();
    for t in defs {
        match choose_target(game, d, seat, t, source, rule, cancellable) {
            Some(chosen) => targets.push(chosen),
            None => return false,
        }
    }
    let target_zcc = targets.iter().map(|ts: &Vec<Target>| ts.iter().map(|t| target_zcc(game, *t)).collect()).collect();
    if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
        top.targets = targets;
        top.target_zcc = target_zcc;
    }
    true
}
/// `PlayerImpl.playAbility`: the ability goes on the stack, then its targets, then mana, then other costs.
/// False: cancelled.
fn activate(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, index: usize) -> bool {
    let def = game.obj(id).def.clone();
    let Some((rule, _, targets, costs, _)) = def.abilities[index].activated() else { return false };
    let zcc = game.obj(id).zcc;
    Arc::make_mut(&mut game.stack).push(StackItem { ability: Some((index, zcc)), ..StackItem::spell(id, seat) });
    if !choose_targets(game, d, seat, targets, None, rule, true) {
        return false;
    }
    for s in 0..game.seats() {
        game.player_mut(s).passed = false;
    }
    if !pay_mana(game, d, seat, &ability_mana_cost(costs)) {
        return false;
    }
    for c in costs {
        match c {
            CostDef::TapSourceCost => game.obj_mut(id).tapped = true,
            CostDef::SacrificeSourceCost => {
                let power = game.power_toughness(id).0;
                if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                    top.lki_power = Some(power);
                }
                game.move_to(id, Zone::Graveyard)
            }
            CostDef::ManaCost { .. } => {}
        }
    }
    true
}

/// `TargetImpl.chooseTarget` and `HumanPlayer.chooseTarget`: one PICK_TARGET per object, until the target has its
/// most or the player is done. Auto-target level 1 (`TargetImpl.tryToAutoChoose`) picks for them when exactly as many
/// legal choices are left as are still needed: never another player, and not their own things when the rule text
/// destroys, exiles, sacrifices or discards. A boolean with fewer than the least chosen cancels a cast (None), and
/// asks again for a trigger; other answers are ignored. None also when a required target has no legal choice.
fn choose_target(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    def: &TargetDef,
    source: Option<ObjId>,
    rule: &str,
    cancellable: bool,
) -> Option<Vec<Target>> {
    let (min, max) = target_counts(def);
    let rule = rule.to_lowercase();
    let feel_bad = ["discard", "sacrifice", "destroy", "exile"].iter().any(|w| rule.contains(w));
    let mut chosen: Vec<Target> = Vec::new();
    let mut selected = false;
    loop {
        let (mut players, mut objs) = possible_targets(game, seat, def);
        // a spell can't target itself (rule 115.5)
        objs.retain(|o| Some(*o) != source);
        if chosen.is_empty() && min > 0 && players.is_empty() && objs.is_empty() {
            return None;
        }
        if chosen.len() >= max {
            return Some(chosen);
        }
        players.retain(|p| !chosen.contains(&Target::Player(*p)));
        objs.retain(|o| !chosen.contains(&Target::Obj(*o)));
        // `isChoiceCompleted`: nothing more to choose, but only once a target was chosen (an "up to" target with
        // nothing to choose is still asked once)
        if selected && players.is_empty() && objs.is_empty() && chosen.len() >= min {
            return Some(chosen);
        }
        if min == max && players.len() + objs.len() == min - chosen.len() && !rule.contains("search") {
            let own_player = players.iter().find(|p| **p == seat).map(|p| Target::Player(*p));
            let auto = own_player
                .filter(|_| !feel_bad)
                .or_else(|| objs.iter().copied().find(|o| !(feel_bad && owns_target(game, seat, *o))).map(Target::Obj));
            if let Some(t) = auto {
                chosen.push(t);
                selected = true;
                continue;
            }
        }
        let q = Query::Target { players: players.clone(), objs: objs.clone(), required: !cancellable };
        match ask(game, d, seat, &q) {
            Answer::Player(s) if players.contains(&s) => {
                chosen.push(Target::Player(s));
                selected = true;
            }
            Answer::Obj(o) if objs.contains(&o) => {
                chosen.push(Target::Obj(o));
                selected = true;
            }
            Answer::Player(s) if chosen.contains(&Target::Player(s)) => chosen.retain(|t| *t != Target::Player(s)),
            Answer::Obj(o) if chosen.contains(&Target::Obj(o)) => chosen.retain(|t| *t != Target::Obj(o)),
            Answer::Bool(_) if chosen.len() >= min => return Some(chosen),
            Answer::Bool(_) if cancellable => return None,
            Answer::Stop => return None,
            _ => {}
        }
    }
}

/// `tryToAutoChoose`'s "own thing": a permanent you control, a card you own, a spell you control.
fn owns_target(game: &Game, seat: Seat, id: ObjId) -> bool {
    let o = game.obj(id);
    match o.zone {
        Zone::Battlefield | Zone::Stack => o.controller == seat,
        _ => o.owner == seat,
    }
}

pub fn possible_targets(game: &Game, seat: Seat, def: &TargetDef) -> (Vec<Seat>, Vec<ObjId>) {
    let players: Vec<Seat> = (0..game.seats()).filter(|s| !game.player(*s).lost).collect();
    let permanents = |f: &dyn Fn(ObjId) -> bool| -> Vec<ObjId> {
        game.battlefield.iter().copied().filter(|id| f(*id) && targetable(game, seat, *id)).collect()
    };
    match def {
        TargetDef::TargetAnyTarget => (
            players,
            permanents(&|id| {
                [CardType::Creature, CardType::Planeswalker, CardType::Battle].iter().any(|t| game.has_type(id, *t))
            }),
        ),
        TargetDef::TargetPlayer => (players, Vec::new()),
        TargetDef::TargetOpponent => (players.into_iter().filter(|s| *s != seat).collect(), Vec::new()),
        TargetDef::TargetPlayerOrPlaneswalker => (players, permanents(&|id| game.has_type(id, CardType::Planeswalker))),
        TargetDef::TargetPermanent { filter, .. } => {
            (Vec::new(), permanents(&|id| game.filter_matches(filter, seat, None, id)))
        }
        TargetDef::TargetSpell { filter } => (
            Vec::new(),
            game.stack
                .iter()
                .rev()
                .filter(|i| i.ability.is_none() && i.delayed.is_none())
                .map(|i| i.card)
                .filter(|c| game.filter_matches(filter, seat, None, *c))
                .collect(),
        ),
        TargetDef::TargetCardInGraveyard { filter, .. } => (
            Vec::new(),
            game.players
                .iter()
                .flat_map(|p| p.graveyard.iter().copied())
                .filter(|c| game.filter_matches(filter, seat, None, *c))
                .collect(),
        ),
    }
}

/// Whether `seat` may target a permanent: not an opponent's with hexproof.
fn targetable(game: &Game, seat: Seat, id: ObjId) -> bool {
    game.obj(id).controller == seat || !game.has_ability(id, &AbilityDef::HexproofAbility)
}

/// How many objects a target takes: at least, at most.
fn target_counts(def: &TargetDef) -> (usize, usize) {
    match def {
        TargetDef::TargetPermanent { min, max, .. } | TargetDef::TargetCardInGraveyard { min, max, .. } => (*min, *max),
        _ => (1, 1),
    }
}

/// Whether a target can be chosen at all (an "up to" target always can).
fn target_possible(game: &Game, seat: Seat, def: &TargetDef) -> bool {
    let (players, objs) = possible_targets(game, seat, def);
    target_counts(def).0 == 0 || !players.is_empty() || !objs.is_empty()
}

fn target_zcc(game: &Game, t: Target) -> u32 {
    match t {
        Target::Obj(o) => game.obj(o).zcc,
        Target::Player(_) => 0,
    }
}

fn target_legal(game: &Game, seat: Seat, def: &TargetDef, t: Target, zcc: u32) -> bool {
    let (players, objs) = possible_targets(game, seat, def);
    match t {
        Target::Player(s) => players.contains(&s),
        Target::Obj(o) => objs.contains(&o) && game.obj(o).zcc == zcc,
    }
}

/// `ManaCostsImpl.pay` with the automatic pool: the pool pays what it can, then the player is asked for a source
/// (`HumanPlayer.playMana`) until the cost is paid or they cancel.
///
/// Mana already floating when the payment starts is "stock" (`ManaPool.setStock`, restricted auto-pay): it's spent
/// only one mana at a time, when the player answers with its type. Mana made during the payment is spent by itself.
fn pay_mana(game: &mut Game, d: &mut dyn Decider, seat: Seat, cost: &ManaCost) -> bool {
    let mut unpaid = cost.clone();
    let mut stock = game.player(seat).mana_pool;
    let mut unlocked = Mana::default();
    loop {
        pay_from_pool(game, seat, &mut unpaid, &mut stock, &mut unlocked);
        if unpaid.mana_value() == 0 {
            return true;
        }
        let sources: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                let o = game.obj(*id);
                o.controller == seat && !o.tapped && o.def.abilities.iter().any(|a| a.mana().is_some())
            })
            .collect();
        let q = Query::PlayMana { cost: unpaid_text(&unpaid), sources };
        match ask(game, d, seat, &q) {
            Answer::Obj(id) => {
                if let Some((_, abilities)) = mana_sources(game, seat).into_iter().find(|(s, _)| *s == id) {
                    let choices = auto_pay_abilities(&abilities, &unpaid);
                    let chosen = if choices.len() == 1 {
                        Some(choices[0])
                    } else {
                        // CHOOSE_ABILITY: an ability of that permanent, or nothing
                        let q =
                            Query::ChooseAbility { source: id, abilities: choices.iter().map(|(i, _)| *i).collect() };
                        match ask(game, d, seat, &q) {
                            Answer::Ability(o, i) if o == id => choices.iter().find(|(j, _)| *j == i).copied(),
                            Answer::Stop => return false,
                            _ => None,
                        }
                    };
                    if let Some((i, m)) = chosen {
                        game.obj_mut(id).tapped = true;
                        let pool = &mut game.player_mut(seat).mana_pool;
                        for c in Color::ALL {
                            pool.colored[c.index()] += m.colored[c.index()];
                        }
                        pool.colorless += m.colorless;
                        // a mana ability's other effects happen at once (a pain land's damage)
                        let effects = game.obj(id).def.abilities[i].mana_effects().to_vec();
                        if !effects.is_empty() {
                            let zcc = game.obj(id).zcc;
                            apply_effects(game, d, &StackItem::spell(id, seat), &effects, &[], (id, zcc));
                        }
                    }
                }
            }
            Answer::ManaType(t) => {
                let k = match t.as_str() {
                    "WHITE" => 0,
                    "BLUE" => 1,
                    "BLACK" => 2,
                    "RED" => 3,
                    "GREEN" => 4,
                    _ => 5,
                };
                *kind(&mut unlocked, k) += 1;
            }
            Answer::Bool(_) | Answer::Stop => return false,
            _ => {}
        }
    }
}

/// One kind of mana: 0–4 are WUBRG, 5 colorless.
fn kind(m: &mut Mana, k: usize) -> &mut u32 {
    if k < 5 {
        &mut m.colored[k]
    } else {
        &mut m.colorless
    }
}

/// Spends what the pool can, but not stock mana that wasn't unlocked: colored costs from matching mana first, then
/// colorless, then generic from what's left (colorless before colors, in WUBRG order). Fresh mana goes before
/// unlocked stock.
fn pay_from_pool(game: &mut Game, seat: Seat, unpaid: &mut ManaCost, stock: &mut Mana, unlocked: &mut Mana) {
    let mut pool = game.player(seat).mana_pool;
    let spend = |pool: &mut Mana, stock: &mut Mana, unlocked: &mut Mana, k: usize, want: u32| -> u32 {
        let fresh = kind(pool, k).saturating_sub(*kind(stock, k));
        let from_fresh = want.min(fresh);
        let from_unlocked = (want - from_fresh).min(*kind(unlocked, k));
        *kind(unlocked, k) -= from_unlocked;
        *kind(stock, k) -= from_unlocked;
        *kind(pool, k) -= from_fresh + from_unlocked;
        from_fresh + from_unlocked
    };
    for k in 0..5 {
        unpaid.colored[k] -= spend(&mut pool, stock, unlocked, k, unpaid.colored[k]);
    }
    unpaid.colorless -= spend(&mut pool, stock, unlocked, 5, unpaid.colorless);
    for k in [5, 0, 1, 2, 3, 4] {
        unpaid.generic -= spend(&mut pool, stock, unlocked, k, unpaid.generic);
    }
    game.player_mut(seat).mana_pool = pool;
}

/// The unpaid part of a cost as Java prints it: `{1}{G}`.
fn unpaid_text(c: &ManaCost) -> String {
    let mut s = String::new();
    if c.generic > 0 {
        s.push_str(&format!("{{{}}}", c.generic));
    }
    for (i, l) in ["W", "U", "B", "R", "G"].iter().enumerate() {
        for _ in 0..c.colored[i] {
            s.push_str(&format!("{{{}}}", l));
        }
    }
    for _ in 0..c.colorless {
        s.push_str("{C}");
    }
    s
}

fn resolve_top(game: &mut Game, d: &mut dyn Decider) {
    let item = match Arc::make_mut(&mut game.stack).pop() {
        Some(i) => i,
        None => return,
    };
    let id = item.card;
    let def = game.obj(id).def.clone();
    if let Some(delayed) = &item.delayed {
        for (o, zcc) in &delayed.sacrifice {
            let p = game.obj(*o);
            if p.zone == Zone::Battlefield && p.zcc == *zcc && p.controller == item.controller {
                game.move_to(*o, Zone::Graveyard);
            }
        }
        return;
    }
    if let Some((index, zcc)) = item.ability {
        if let Some(a) = def.abilities.get(index) {
            if !game.condition_holds(a.intervening_if(), item.controller, id) {
                return;
            }
        }
        let Some((rule, effects, targets, optional)) = def.abilities.get(index).and_then(|a| a.on_stack()) else {
            return;
        };
        // "may" abilities ask at resolution (`TriggeredAbilityImpl.resolve`)
        if optional {
            let q = Query::YesNo { message: rule.to_string() };
            loop {
                match ask(game, d, item.controller, &q) {
                    Answer::Bool(true) => break,
                    Answer::Bool(false) | Answer::Stop => return,
                    _ => {}
                }
            }
        }
        if let Some(legal) = legal_targets(game, &item, targets) {
            let effects = effects.to_vec();
            apply_effects(game, d, &item, &effects, &legal, (id, zcc));
        }
        return;
    }
    if def.is_permanent_card() {
        enter_battlefield(game, d, id, item.controller, item.kicked);
        return;
    }
    let (effects, targets) = def.spell.mode(item.mode);
    if let Some(legal) = legal_targets(game, &item, targets) {
        let zcc = game.obj(id).zcc;
        apply_effects(game, d, &item, effects, &legal, (id, zcc));
    }
    if game.obj(id).zone == Zone::Stack {
        game.move_to(id, Zone::Graveyard);
    }
}

/// The chosen targets still legal, per target; None when it had targets and all are illegal, so it doesn't resolve
/// (rule 608.2b).
fn legal_targets(game: &Game, item: &StackItem, defs: &[TargetDef]) -> Option<Vec<Vec<Target>>> {
    // no targets, but a fixed target pointer (the permanent that entered): it's acted on while it's the same object
    if defs.is_empty() && !item.targets.is_empty() {
        let fixed = item
            .targets
            .iter()
            .zip(item.target_zcc.iter())
            .map(|(ts, zs)| {
                ts.iter()
                    .zip(zs.iter())
                    .filter(|(t, z)| match t {
                        Target::Obj(o) => game.obj(*o).zcc == **z,
                        Target::Player(_) => true,
                    })
                    .map(|(t, _)| *t)
                    .collect()
            })
            .collect();
        return Some(fixed);
    }
    let mut any_chosen = false;
    let mut any_legal = false;
    let legal: Vec<Vec<Target>> = defs
        .iter()
        .enumerate()
        .map(|(i, def)| {
            let chosen = item.targets.get(i).cloned().unwrap_or_default();
            let zccs = item.target_zcc.get(i).cloned().unwrap_or_default();
            any_chosen |= !chosen.is_empty();
            let ok: Vec<Target> = chosen
                .into_iter()
                .zip(zccs)
                .filter(|(t, z)| target_legal(game, item.controller, def, *t, *z))
                .map(|(t, _)| t)
                .collect();
            any_legal |= !ok.is_empty();
            ok
        })
        .collect();
    if any_chosen && !any_legal {
        None
    } else {
        Some(legal)
    }
}

/// A resolving spell's or ability's effects, in order. Target effects act on the first target, if it's still legal.
fn apply_effects(
    game: &mut Game,
    d: &mut dyn Decider,
    item: &StackItem,
    effects: &[EffectDef],
    legal: &[Vec<Target>],
    source: (ObjId, u32),
) {
    let legal_target = |i: usize| legal.get(i).and_then(|ts| ts.first().copied());
    let target = legal_target(0);
    // the first target's objects (an effect acts on every one, as Java's first target pointer)
    let objs: Vec<ObjId> = legal
        .first()
        .map(|ts| ts.iter().filter_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }).collect())
        .unwrap_or_default();
    let players: Vec<Seat> = legal
        .first()
        .map(|ts| ts.iter().filter_map(|t| if let Target::Player(p) = t { Some(*p) } else { None }).collect())
        .unwrap_or_default();
    let controller = item.controller;
    for e in effects {
        match e {
            EffectDef::DamageTargetEffect { amount } => {
                if let Some(t) = target {
                    deal_damage(game, source.0, t, *amount, false);
                }
            }
            EffectDef::DrawCardSourceControllerEffect { amount } => draw(game, controller, *amount as usize),
            EffectDef::GainLifeEffect { amount } => gain_life(game, controller, *amount),
            EffectDef::DestroyTargetEffect => {
                for o in &objs {
                    destroy(game, *o);
                }
            }
            EffectDef::CounterTargetEffect => {
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone == Zone::Stack {
                        Arc::make_mut(&mut game.stack).retain(|i| i.card != o || i.ability.is_some());
                        game.move_to(o, Zone::Graveyard);
                    }
                }
            }
            EffectDef::BoostTargetEffect { power, toughness, duration } => {
                for o in &objs {
                    boost(game, *o, *power, *toughness, *duration);
                }
            }
            EffectDef::BoostSourceEffect { power, toughness, duration } => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    boost(game, o, *power, *toughness, *duration);
                }
            }
            EffectDef::AddCountersSourceEffect { counter } => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    game.obj_mut(o).add_counters(&counter.name, counter.amount);
                }
            }
            EffectDef::AddCountersTargetEffect { counter } => {
                for o in &objs {
                    game.obj_mut(*o).add_counters(&counter.name, counter.amount);
                }
            }
            EffectDef::CreateTokenEffect { token, amount, tapped_and_attacking, sacrifice_at_end_of_combat } => {
                let def = Arc::new(token.card());
                let mut created = Vec::new();
                for _ in 0..*amount {
                    let id = game.create_obj(def.clone(), controller, Zone::Library);
                    let o = game.obj_mut(id);
                    o.token = true;
                    // registered after every card
                    o.trigger_order = (1 << 32) + id.0 as u64;
                    enter_battlefield(game, d, id, controller, false);
                    if *tapped_and_attacking {
                        let defender = game.opponent(controller);
                        let o = game.obj_mut(id);
                        o.tapped = true;
                        o.attacking = Some(defender);
                    }
                    created.push((id, game.obj(id).zcc));
                }
                if *sacrifice_at_end_of_combat {
                    let delayed =
                        Delayed { rule: "At end of combat, sacrifice that token.".into(), sacrifice: created };
                    game.at_end_of_combat
                        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
                }
            }
            EffectDef::DamageTargetSourcePowerEffect => {
                if let Some(t) = target {
                    let power = item.lki_power.unwrap_or_else(|| game.power_toughness(source.0).0).max(0);
                    deal_damage(game, source.0, t, power, false);
                }
            }
            EffectDef::ConditionalOneShotEffect { effect, otherwise, condition } => {
                let holds = condition_with_kicker(game, condition, controller, source.0, item.kicked);
                if let Some(e) = if holds { effect } else { otherwise } {
                    apply_effects(game, d, item, std::slice::from_ref(&**e), legal, source);
                }
            }
            EffectDef::DamagePlayersEffect { amount, controller: who } => {
                for s in 0..game.seats() {
                    if (who == "opponent" && s != controller) || who == "any" {
                        deal_damage(game, source.0, Target::Player(s), *amount, false);
                    }
                }
            }
            EffectDef::SacrificeSourceEffect => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield
                    && game.obj(o).zcc == zcc
                    && game.obj(o).controller == controller
                {
                    game.move_to(o, Zone::Graveyard);
                }
            }
            EffectDef::DoIfCostPaid { effect, cost } => {
                let q = Query::YesNo { message: format!("Pay {}?", ability_mana_cost(cost).text) };
                let pay = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if pay {
                    let before = game.clone();
                    if pay_mana(game, d, controller, &ability_mana_cost(cost)) {
                        apply_effects(game, d, item, std::slice::from_ref(&**effect), legal, source);
                    } else if !d.stopped() {
                        // `DoIfCostPaid` undoes a payment that was cancelled
                        *game = before;
                    }
                }
            }
            EffectDef::DamageWithPowerFromOneToAnotherTargetEffect => {
                if let (Some(Target::Obj(from)), Some(to)) = (target, legal_target(1)) {
                    let (power, _) = game.power_toughness(from);
                    deal_damage(game, from, to, power, false);
                }
            }
            // a resolving one boosts the creatures it applies to now; a static one applies while its source is on the
            // battlefield (`Game::power_toughness`)
            EffectDef::BoostControlledEffect {
                power,
                toughness,
                duration: Duration::EndOfTurn,
                filter,
                exclude_source,
            } => {
                for id in Arc::clone(&game.battlefield).iter().copied() {
                    if game.obj(id).controller == controller
                        && game.filter_matches(filter, controller, Some(source.0), id)
                        && !(*exclude_source && id == source.0)
                    {
                        boost(game, id, *power, *toughness, Duration::EndOfTurn);
                    }
                }
            }
            EffectDef::GainAbilityControlledEffect {
                ability,
                duration: Duration::EndOfTurn,
                filter,
                exclude_source,
            } => {
                for id in Arc::clone(&game.battlefield).iter().copied() {
                    if game.obj(id).controller == controller
                        && game.filter_matches(filter, controller, Some(source.0), id)
                        && !(*exclude_source && id == source.0)
                    {
                        grant(game, id, ability);
                    }
                }
            }
            EffectDef::GainAbilityAllEffect { ability, duration: Duration::EndOfTurn, filter, exclude_source } => {
                for id in Arc::clone(&game.battlefield).iter().copied() {
                    if game.filter_matches(filter, controller, Some(source.0), id)
                        && !(*exclude_source && id == source.0)
                    {
                        grant(game, id, ability);
                    }
                }
            }
            EffectDef::GainAbilityTargetEffect { ability, .. } => {
                for o in &objs {
                    grant(game, *o, ability);
                }
            }
            EffectDef::LoseLifeOpponentsEffect { amount } => {
                for s in 0..game.seats() {
                    if s != controller {
                        game.player_mut(s).life -= amount;
                    }
                }
            }
            EffectDef::LoseLifeSourceControllerEffect { amount } => game.player_mut(controller).life -= amount,
            EffectDef::LoseLifeTargetEffect { amount } => {
                for s in &players {
                    game.player_mut(*s).life -= amount;
                }
            }
            EffectDef::DrawCardTargetEffect { amount } => {
                for s in &players {
                    draw(game, *s, *amount as usize);
                }
            }
            EffectDef::DestroyAllEffect { filter } => {
                for o in matching(game, filter, controller, source.0) {
                    destroy(game, o);
                }
            }
            EffectDef::DamageAllEffect { amount, filter } => {
                for o in matching(game, filter, controller, source.0) {
                    deal_damage(game, source.0, Target::Obj(o), *amount, false);
                }
            }
            EffectDef::BoostAllEffect { power, toughness, duration: Duration::EndOfTurn, filter, exclude_source } => {
                for o in matching(game, filter, controller, source.0) {
                    if !(*exclude_source && o == source.0) {
                        boost(game, o, *power, *toughness, Duration::EndOfTurn);
                    }
                }
            }
            EffectDef::AddCountersAllEffect { counter, filter } => {
                for o in matching(game, filter, controller, source.0) {
                    game.obj_mut(o).add_counters(&counter.name, counter.amount);
                }
            }
            EffectDef::ExileTargetEffect => {
                for o in &objs {
                    if game.obj(*o).zone == Zone::Stack {
                        Arc::make_mut(&mut game.stack).retain(|i| i.card != *o || i.ability.is_some());
                    }
                    game.move_to(*o, Zone::Exile);
                }
            }
            EffectDef::ReturnToHandTargetEffect => {
                for o in &objs {
                    if game.obj(*o).zone == Zone::Stack {
                        Arc::make_mut(&mut game.stack).retain(|i| i.card != *o || i.ability.is_some());
                    }
                    game.move_to(*o, Zone::Hand);
                }
            }
            EffectDef::TapTargetEffect => {
                for o in &objs {
                    game.obj_mut(*o).tapped = true;
                }
            }
            EffectDef::UntapTargetEffect => {
                for o in &objs {
                    game.obj_mut(*o).tapped = false;
                }
            }
            EffectDef::TapSourceEffect => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    game.obj_mut(o).tapped = true;
                }
            }
            EffectDef::DamageControllerEffect { amount } => {
                deal_damage(game, source.0, Target::Player(controller), *amount, false);
            }
            EffectDef::ScryEffect { amount } => {
                let cards: Vec<ObjId> =
                    game.player(controller).library.iter().take(*amount as usize).copied().collect();
                if !cards.is_empty() {
                    let rule = item_rule(game, item);
                    let Some(bottom) = choose_cards(game, d, controller, &cards, &cards, 0, cards.len(), &rule) else {
                        return;
                    };
                    let top: Vec<ObjId> = cards.iter().copied().filter(|c| !bottom.contains(c)).collect();
                    if !put_on_library(game, d, controller, &bottom, false, true) {
                        return;
                    }
                    if !put_on_library(game, d, controller, &top, true, true) {
                        return;
                    }
                }
            }
            EffectDef::SurveilEffect { amount } => {
                let cards: Vec<ObjId> =
                    game.player(controller).library.iter().take(*amount as usize).copied().collect();
                if !cards.is_empty() {
                    let rule = item_rule(game, item);
                    let Some(yard) = choose_cards(game, d, controller, &cards, &cards, 0, cards.len(), &rule) else {
                        return;
                    };
                    for c in &yard {
                        game.move_to(*c, Zone::Graveyard);
                    }
                    let top: Vec<ObjId> = cards.iter().copied().filter(|c| !yard.contains(c)).collect();
                    if !put_on_library(game, d, controller, &top, true, true) {
                        return;
                    }
                }
            }
            EffectDef::LookLibraryAndPickControllerEffect {
                look,
                look_count,
                pick,
                filter,
                picked,
                looked,
                up_to,
                optional,
            } => {
                let n = match look_count {
                    Some(c) => count(game, c, controller),
                    None => *look,
                };
                let cards: Vec<ObjId> =
                    game.player(controller).library.iter().take(n.max(0) as usize).copied().collect();
                let fitting: Vec<ObjId> = cards
                    .iter()
                    .copied()
                    .filter(|c| game.filter_matches(filter, controller, Some(source.0), *c))
                    .collect();
                let number = (*pick as usize).min(fitting.len());
                let mut chosen = Vec::new();
                if number >= 1 {
                    let go = !*optional || {
                        let q = Query::YesNo { message: "Put a card?".into() };
                        loop {
                            match ask(game, d, controller, &q) {
                                Answer::Bool(b) => break b,
                                Answer::Stop => return,
                                _ => {}
                            }
                        }
                    };
                    if go {
                        let rule = item_rule(game, item);
                        let min = if *up_to { 0 } else { number };
                        let Some(c) = choose_cards(game, d, controller, &cards, &fitting, min, number, &rule) else {
                            return;
                        };
                        chosen = c;
                    }
                }
                let rest: Vec<ObjId> = cards.iter().copied().filter(|c| !chosen.contains(c)).collect();
                if !put_cards(game, d, controller, &chosen, *picked) || !put_cards(game, d, controller, &rest, *looked)
                {
                    return;
                }
            }
            EffectDef::CounterUnlessPaysEffect { cost } => {
                let Some(Target::Obj(spell)) = target else { continue };
                if game.obj(spell).zone != Zone::Stack {
                    continue;
                }
                let payer = game.stack.iter().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.controller);
                let Some(payer) = payer else { continue };
                let mana = ability_mana_cost(cost);
                // `chooseUse` "Pay {2}?" (a mana cost's `canPay` is always true), then the payment; anything short of
                // paying counters it
                let paid = {
                    let q = Query::YesNo { message: format!("Pay {}?", mana.text) };
                    let yes = loop {
                        match ask(game, d, payer, &q) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    yes && pay_mana(game, d, payer, &mana)
                };
                if d.stopped() {
                    return;
                }
                if !paid {
                    Arc::make_mut(&mut game.stack).retain(|i| i.card != spell || i.ability.is_some());
                    game.move_to(spell, Zone::Graveyard);
                }
            }
            EffectDef::ExileTargetIfDiesEffect => {
                if let Some(Target::Obj(o)) = target {
                    let zcc = game.obj(o).zcc;
                    game.exile_if_dies.push((o, zcc));
                }
            }
            // static abilities (`Game::power_toughness`, `Game::has_ability`) and as-enters effects
            // (`enter_battlefield`)
            EffectDef::BoostControlledEffect { .. }
            | EffectDef::GainAbilityAllEffect { .. }
            | EffectDef::GainAbilityControlledEffect { .. }
            | EffectDef::GainAbilitySourceEffect { .. }
            | EffectDef::ConditionalContinuousEffect { .. }
            | EffectDef::SetBasePowerToughnessSourceEffect { .. }
            | EffectDef::AddCardSubTypeSourceEffect
            | EffectDef::BoostAllEffect { .. }
            | EffectDef::TapSourceUnlessPaysEffect { .. } => {}
        }
    }
}

/// A spell's or ability's rule text (auto-targeting reads it).
fn item_rule(game: &Game, item: &StackItem) -> String {
    let def = &game.obj(item.card).def;
    match item.ability {
        Some((i, _)) => {
            def.abilities.get(i).and_then(|a| a.on_stack()).map(|(r, ..)| r.to_string()).unwrap_or_default()
        }
        None => def.spell.rule.clone(),
    }
}

/// A count (Java's `DynamicValue`s) for `you`.
fn count(game: &Game, name: &str, you: Seat) -> i32 {
    let mine = |t: CardType| {
        game.battlefield.iter().filter(|b| game.obj(**b).controller == you && game.has_type(**b, t)).count() as i32
    };
    match name {
        "creatures_you_control" => mine(CardType::Creature),
        "lands_you_control" => mine(CardType::Land),
        _ => 0,
    }
}

/// `HumanPlayer.chooseTarget(outcome, cards, target, …)`: cards from `shown`, of which `possible` can be chosen,
/// between `min` and `max` of them, one PICK_TARGET each until the most are chosen or the player is done (a
/// boolean, once at least `min` are chosen). Auto-choice as `TargetImpl.tryToAutoChoose`. None: the decider stopped.
#[allow(clippy::too_many_arguments)]
fn choose_cards(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    shown: &[ObjId],
    possible: &[ObjId],
    min: usize,
    max: usize,
    rule: &str,
) -> Option<Vec<ObjId>> {
    let _ = shown;
    let rule = rule.to_lowercase();
    let feel_bad = ["discard", "sacrifice", "destroy", "exile"].iter().any(|w| rule.contains(w));
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        if chosen.len() >= max {
            return Some(chosen);
        }
        let left: Vec<ObjId> = possible.iter().copied().filter(|c| !chosen.contains(c)).collect();
        if min == max && left.len() == min - chosen.len() && !rule.contains("search") {
            if let Some(c) = left.iter().copied().find(|c| !(feel_bad && owns_target(game, seat, *c))) {
                chosen.push(c);
                continue;
            }
        }
        let required = chosen.len() < min && !left.is_empty();
        match ask(game, d, seat, &Query::Cards { objs: left.clone(), required }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if left.contains(&o) => chosen.push(o),
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ if chosen.len() >= min || !required => return Some(chosen),
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}

/// `PlayerImpl.putCardsOnTopOfLibrary` / `putCardsOnBottomOfLibrary`. In any order: Java picks a card at random to
/// learn the owner (a random number when there are two or more), then the player picks one card at a time (HP.choose,
/// required) until one is left; the last chosen ends up topmost (bottommost). In a random order: shuffled. False:
/// the decider stopped.
fn put_on_library(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    cards: &[ObjId],
    top: bool,
    any_order: bool,
) -> bool {
    if cards.is_empty() {
        return true;
    }
    let mut cards = cards.to_vec();
    if !any_order {
        game.rng.shuffle(&mut cards);
        for c in cards {
            to_library(game, c, top);
        }
        return true;
    }
    if cards.len() >= 2 {
        game.rng.next_int_bound(cards.len() as i32);
    }
    while cards.len() > 1 {
        let picked = loop {
            match ask(game, d, seat, &Query::Cards { objs: cards.clone(), required: true }) {
                Answer::Obj(o) if cards.contains(&o) => break o,
                Answer::Stop => return false,
                _ => {}
            }
        };
        cards.retain(|c| *c != picked);
        to_library(game, picked, top);
    }
    for c in cards {
        to_library(game, c, top);
    }
    true
}

/// Moves a card to the top or bottom of its owner's library.
fn to_library(game: &mut Game, id: ObjId, top: bool) {
    game.move_to(id, Zone::Library);
    if top {
        let owner = game.obj(id).owner;
        let lib = &mut game.player_mut(owner).library;
        lib.retain(|x| *x != id);
        lib.insert(0, id);
    }
}

/// `PutCards.moveCards`. False: the decider stopped.
fn put_cards(game: &mut Game, d: &mut dyn Decider, seat: Seat, cards: &[ObjId], to: crate::card::PutCards) -> bool {
    use crate::card::PutCards as P;
    match to {
        P::TopAny => put_on_library(game, d, seat, cards, true, true),
        P::BottomAny => put_on_library(game, d, seat, cards, false, true),
        P::BottomRandom => put_on_library(game, d, seat, cards, false, false),
        P::Hand => {
            for c in cards {
                game.move_to(*c, Zone::Hand);
            }
            true
        }
        P::Graveyard => {
            for c in cards {
                game.move_to(*c, Zone::Graveyard);
            }
            true
        }
        P::Exiled => {
            for c in cards {
                game.move_to(*c, Zone::Exile);
            }
            true
        }
        P::Battlefield | P::BattlefieldTapped => {
            for c in cards {
                enter_battlefield(game, d, *c, seat, false);
                if to == P::BattlefieldTapped {
                    game.obj_mut(*c).tapped = true;
                }
            }
            !d.stopped()
        }
    }
}

/// A condition where it's known whether the spell was kicked.
fn condition_with_kicker(game: &Game, c: &Condition, you: Seat, source: ObjId, kicked: bool) -> bool {
    match c {
        Condition::Kicked => kicked,
        _ => game.condition_holds(Some(c), you, source),
    }
}

/// Destroys a permanent, unless it's indestructible.
fn destroy(game: &mut Game, id: ObjId) {
    if game.obj(id).zone == Zone::Battlefield && !game.has_ability(id, &AbilityDef::IndestructibleAbility) {
        game.move_to(id, Zone::Graveyard);
    }
}

/// The permanents of a filter, as `getActivePermanents` lists them (battlefield order).
fn matching(game: &Game, filter: &crate::card::Filter, you: Seat, source: ObjId) -> Vec<ObjId> {
    game.battlefield.iter().copied().filter(|id| game.filter_matches(filter, you, Some(source), *id)).collect()
}

/// An ability until end of turn (the only duration resolving grants have yet).
fn grant(game: &mut Game, target: ObjId, ability: &AbilityDef) {
    let zcc = game.obj(target).zcc;
    Arc::make_mut(&mut game.grants).push(Grant { target, zcc, ability: ability.clone(), until_end_of_turn: true });
}

fn boost(game: &mut Game, target: ObjId, power: i32, toughness: i32, duration: Duration) {
    let zcc = game.obj(target).zcc;
    Arc::make_mut(&mut game.boosts).push(Boost {
        target,
        zcc,
        power,
        toughness,
        until_end_of_turn: duration == Duration::EndOfTurn,
    });
}

/// A permanent enters: under `controller`, tapped if it says so, with what its "enters with" abilities give it; then
/// its own enters triggers and other permanents' "whenever … enters" triggers wait to go on the stack.
fn enter_battlefield(game: &mut Game, d: &mut dyn Decider, id: ObjId, controller: Seat, kicked: bool) {
    game.move_to(id, Zone::Battlefield);
    let def = game.obj(id).def.clone();
    let zcc = {
        let o = game.obj_mut(id);
        o.controller = controller;
        if def.abilities.iter().any(|a| a.is_enters_tapped()) {
            o.tapped = true;
        }
        o.zcc
    };
    // shock lands: `TapSourceUnlessPaysEffect` asks only when the life can be paid
    for a in &def.abilities {
        if let AbilityDef::AsEntersBattlefieldAbility { effects, .. } = a {
            for e in effects {
                if let EffectDef::TapSourceUnlessPaysEffect { life } = e {
                    let paid = game.player(controller).life >= *life && {
                        let q = Query::YesNo {
                            message: format!("Pay {} life? (otherwise {} becomes tapped)", life, def.name),
                        };
                        loop {
                            match ask(game, d, controller, &q) {
                                Answer::Bool(b) => break b,
                                Answer::Stop => return,
                                _ => {}
                            }
                        }
                    };
                    if paid {
                        game.player_mut(controller).life -= life;
                    } else {
                        game.obj_mut(id).tapped = true;
                    }
                }
            }
        }
    }
    for a in &def.abilities {
        if let AbilityDef::EntersBattlefieldAbility { effects, condition, .. } = a {
            if condition.as_ref().is_none_or(|c| condition_with_kicker(game, c, controller, id, kicked)) {
                let item = StackItem::spell(id, controller);
                apply_effects(game, d, &item, effects, &[], (id, zcc));
            }
        }
    }
    // everything that triggers on this event, in Java's registration order
    let mut sources: Vec<ObjId> = game.battlefield.iter().copied().filter(|s| *s != id).collect();
    sources.insert(0, id);
    let mut fired = game.triggers(sources, |g, src, you, t| match t {
        Trigger::Enters | Trigger::EntersOrDies | Trigger::EntersOrAttacks => src == id,
        Trigger::EntersAll { filter } => g.filter_matches(filter, you, Some(src), id),
        _ => false,
    });
    for t in fired.iter_mut() {
        let a = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i).cloned());
        if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. })) {
            t.targets = vec![vec![Target::Obj(id)]];
            t.target_zcc = vec![vec![zcc]];
        }
    }
    game.pending.extend(fired);
}

/// `GameImpl.checkStateAndTriggered`: state-based actions until none apply, then waiting triggers go on the stack,
/// active player's first (APNAP), and again until nothing changes.
fn check_state_and_triggered(game: &mut Game, d: &mut dyn Decider) {
    loop {
        state_based_actions(game);
        if game.over || d.stopped() {
            return;
        }
        if legend_rule(game, d) {
            continue;
        }
        if d.stopped() || game.pending.is_empty() {
            return;
        }
        let mut pending = std::mem::take(&mut game.pending);
        // an intervening "if" is checked as it triggers (and again as it resolves)
        pending.retain(|t| {
            let ability = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i));
            game.condition_holds(ability.and_then(|a| a.intervening_if()), t.controller, t.card)
        });
        let active = game.active.unwrap_or(0);
        for k in 0..game.seats() {
            let seat = (active + k) % game.seats();
            let mut mine: Vec<StackItem> = pending.iter().filter(|t| t.controller == seat).cloned().collect();
            while !mine.is_empty() {
                // the player orders their triggers, unless they're all the same (auto-order)
                let pick = if mine.len() > 1 && !mine.iter().all(|t| same_trigger(game, t, &mine[0])) {
                    match ask(game, d, seat, &Query::PickTrigger { count: mine.len() }) {
                        Answer::Trigger(i) if i < mine.len() => i,
                        Answer::Stop => return,
                        _ => continue,
                    }
                } else {
                    0
                };
                let t = mine.remove(pick);
                let ability = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i).cloned());
                Arc::make_mut(&mut game.stack).push(t);
                // its targets, which can't be cancelled; with no legal ones it's removed (rule 603.3d)
                if let Some((rule, _, targets, _)) = ability.as_ref().and_then(|a| a.triggered()) {
                    if !targets.is_empty() && !choose_targets(game, d, seat, targets, None, rule, false) {
                        if d.stopped() {
                            return;
                        }
                        Arc::make_mut(&mut game.stack).pop();
                    }
                }
            }
        }
    }
}

fn same_trigger(game: &Game, a: &StackItem, b: &StackItem) -> bool {
    game.stack_name(a) == game.stack_name(b) && a.targets == b.targets
}

/// Damage from `source`: to a player it's life lost, to a creature it's marked. Lifelink gains its controller life.
/// Returns combat lifelink's life, which `combat_damage` gains once per source.
fn deal_damage(game: &mut Game, source: ObjId, to: Target, amount: i32, combat: bool) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let mut lifelink = 0;
    let src = game.obj(source).clone();
    match to {
        Target::Player(s) => {
            game.player_mut(s).life -= amount;
            if !combat {
                damaged_opponent_triggers(game, s);
            }
        }
        Target::Obj(o) => {
            if game.obj(o).zone != Zone::Battlefield {
                return 0;
            }
            let deathtouch = src.has(&AbilityDef::DeathtouchAbility);
            let t = game.obj_mut(o);
            t.damage += amount;
            if deathtouch {
                t.deathtouched = true;
            }
        }
    }
    if src.has(&AbilityDef::LifelinkAbility) {
        if combat {
            lifelink = amount;
        } else {
            gain_life(game, src.controller, amount);
        }
    }
    if combat {
        if let Target::Player(p) = to {
            let fired = game.triggers([source], |_, _, _, t| matches!(t, Trigger::CombatDamageToPlayer));
            for mut t in fired {
                let a = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i));
                if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. })) {
                    t.targets = vec![vec![Target::Player(p)]];
                    t.target_zcc = vec![vec![0]];
                }
                game.pending.push(t);
            }
        }
    }
    lifelink
}

/// Noncombat damage to `seat`: "whenever one or more opponents are dealt noncombat damage" triggers, once per batch
/// (a resolution's damage is one batch: a trigger already waiting isn't added again).
fn damaged_opponent_triggers(game: &mut Game, seat: Seat) {
    let sources: Vec<ObjId> = game.battlefield.iter().copied().collect();
    let fired = game.triggers(sources, |g, src, you, t| {
        matches!(t, Trigger::OpponentsDealtNoncombatDamage)
            && you != seat
            && !g.pending.iter().any(|p| p.card == src && same_ability(g, p, t))
    });
    game.pending.extend(fired);
}

fn same_ability(game: &Game, p: &StackItem, t: &Trigger) -> bool {
    p.ability.and_then(|(i, _)| game.obj(p.card).def.abilities.get(i)).and_then(|a| a.trigger()) == Some(t)
}

/// `seat` gains life: "whenever you gain life" triggers (`GAINED_LIFE`).
pub fn gain_life(game: &mut Game, seat: Seat, amount: i32) {
    if amount <= 0 {
        return;
    }
    game.player_mut(seat).life += amount;
    let sources: Vec<ObjId> = game.battlefield.iter().copied().collect();
    let fired = game.triggers(sources, |_, _, you, t| matches!(t, Trigger::YouGainLife) && you == seat);
    game.pending.extend(fired);
}

// ------------------------------------------------------------------------------------------------------------ combat

pub fn attackers(game: &Game) -> Vec<ObjId> {
    game.battlefield.iter().copied().filter(|id| game.obj(*id).attacking.is_some()).collect()
}

fn first_strike_in_combat(game: &Game) -> bool {
    game.battlefield.iter().any(|id| {
        let o = game.obj(*id);
        (o.attacking.is_some() || !o.blocking.is_empty())
            && (game.has_ability(*id, &AbilityDef::FirstStrikeAbility)
                || game.has_ability(*id, &AbilityDef::DoubleStrikeAbility))
    })
}

fn can_attack(game: &Game, seat: Seat, id: ObjId) -> bool {
    let o = game.obj(id);
    o.controller == seat
        && o.is(CardType::Creature)
        && !o.tapped
        && o.attacking.is_none()
        && (!o.sick || o.has(&AbilityDef::HasteAbility))
        && !o.has(&AbilityDef::DefenderAbility)
}

/// `Combat.selectAttackers` and `HumanPlayer.selectAttackers`: asked only when something can attack; "special" is
/// the All attack button, a creature toggles it, any other answer is done.
fn declare_attackers(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
    let defender = game.opponent(seat);
    let possible =
        |g: &Game| -> Vec<ObjId> { g.battlefield.iter().copied().filter(|id| can_attack(g, seat, *id)).collect() };
    if possible(game).is_empty() {
        return;
    }
    loop {
        let creatures = possible(game);
        match ask(game, d, seat, &Query::Attackers { creatures: creatures.clone() }) {
            Answer::Text(s) if s == "special" => {
                for id in creatures {
                    attack(game, id, defender);
                }
            }
            Answer::Obj(id) => {
                if creatures.contains(&id) {
                    attack(game, id, defender);
                } else if game.obj(id).attacking.is_some() && game.stack.is_empty() {
                    let vigilance = game.obj(id).has(&AbilityDef::VigilanceAbility);
                    let o = game.obj_mut(id);
                    o.attacking = None;
                    if !vigilance {
                        o.tapped = false;
                    }
                }
            }
            _ => return,
        }
        if d.stopped() {
            return;
        }
    }
}

fn attack(game: &mut Game, id: ObjId, defender: Seat) {
    let vigilance = game.obj(id).has(&AbilityDef::VigilanceAbility);
    let o = game.obj_mut(id);
    o.attacking = Some(defender);
    if !vigilance {
        o.tapped = true;
    }
}

fn can_block(game: &Game, blocker: ObjId, attacker: ObjId) -> bool {
    if game.has_ability(attacker, &AbilityDef::FlyingAbility)
        && !game.has_ability(blocker, &AbilityDef::FlyingAbility)
        && !game.has_ability(blocker, &AbilityDef::ReachAbility)
    {
        return false;
    }
    true
}

/// `HumanPlayer.selectBlockers`: a creature, then the attacker it blocks (picked for you when there's one); done ends.
/// `Combat.selectBlockers`: the defender declares, then the block restrictions are checked. A creature with menace
/// blocked by one loses its blocker; if the defender could have blocked it with two, they declare again.
fn declare_blockers(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
    loop {
        select_blockers(game, d, seat);
        if d.stopped() {
            return;
        }
        let mut legal = true;
        for a in attackers(game) {
            if !game.has_ability(a, &AbilityDef::MenaceAbility) {
                continue;
            }
            let blockers = live_blockers(game, a);
            if blockers.len() != 1 {
                continue;
            }
            game.obj_mut(blockers[0]).blocking.clear();
            let o = game.obj_mut(a);
            o.blocked_by.clear();
            o.blocked = false;
            let possible = game
                .battlefield
                .iter()
                .filter(|b| {
                    let o = game.obj(**b);
                    o.controller == seat && o.is(CardType::Creature) && !o.tapped && can_block(game, **b, a)
                })
                .count();
            if possible >= 2 {
                legal = false;
            }
        }
        if legal {
            return;
        }
    }
}

fn select_blockers(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
    let free = |g: &Game| -> Vec<ObjId> {
        g.battlefield
            .iter()
            .copied()
            .filter(|id| {
                let o = g.obj(*id);
                o.controller == seat && o.is(CardType::Creature) && !o.tapped && o.blocking.is_empty()
            })
            .collect()
    };
    if free(game).is_empty() {
        return;
    }
    loop {
        let all = attackers(game);
        let creatures: Vec<ObjId> =
            free(game).into_iter().filter(|b| all.iter().any(|a| can_block(game, *b, *a))).collect();
        let answer = ask(game, d, seat, &Query::Blockers { creatures });
        let blocker = match answer {
            Answer::Obj(id) => id,
            _ => return,
        };
        if d.stopped() {
            return;
        }
        let o = game.obj(blocker);
        if o.controller != seat || !o.is(CardType::Creature) || o.zone != Zone::Battlefield {
            continue;
        }
        if !o.blocking.is_empty() {
            if game.stack.is_empty() {
                game.obj_mut(blocker).blocking.clear();
                refresh_blocked(game);
            }
            continue;
        }
        if o.tapped {
            continue;
        }
        let options: Vec<ObjId> = attackers(game).into_iter().filter(|a| can_block(game, blocker, *a)).collect();
        let chosen = if options.len() == 1 {
            Some(options[0])
        } else {
            match ask(game, d, seat, &Query::BlockTarget { attackers: options.clone() }) {
                Answer::Obj(a) if options.contains(&a) => Some(a),
                _ => None,
            }
        };
        if let Some(a) = chosen {
            game.obj_mut(blocker).blocking.push(a);
            let by = &mut game.obj_mut(a).blocked_by;
            by.retain(|x| *x != blocker);
            by.push(blocker);
            game.obj_mut(a).blocked = true;
        }
        if d.stopped() {
            return;
        }
    }
}

fn refresh_blocked(game: &mut Game) {
    for a in attackers(game) {
        let blocked = game.battlefield.iter().any(|b| game.obj(*b).blocking.contains(&a));
        game.obj_mut(a).blocked = blocked;
    }
}

fn deals_damage_in(game: &Game, id: ObjId, first: bool, has_first_step: bool) -> bool {
    let fs = game.has_ability(id, &AbilityDef::FirstStrikeAbility);
    let ds = game.has_ability(id, &AbilityDef::DoubleStrikeAbility);
    if first {
        fs || ds
    } else {
        !has_first_step || ds || !fs
    }
}

/// The blockers still blocking an attacker, in declaration order.
fn live_blockers(game: &Game, a: ObjId) -> Vec<ObjId> {
    game.obj(a)
        .blocked_by
        .iter()
        .copied()
        .filter(|b| game.obj(*b).zone == Zone::Battlefield && game.obj(*b).blocking.contains(&a))
        .collect()
}

/// Damage that's lethal to `blocker` from `attacker` (`Permanent.getLethalDamage`).
fn lethal_damage(game: &Game, blocker: ObjId, attacker: ObjId) -> i32 {
    let (_, t) = game.power_toughness(blocker);
    let lethal = (t - game.obj(blocker).damage).max(0);
    if game.obj(attacker).has(&AbilityDef::DeathtouchAbility) {
        lethal.min(1)
    } else {
        lethal
    }
}

/// `HumanPlayer.getMultiAmountWithIndividualConstraints`: asked unless there's nothing to choose; only a string of
/// good values ends it.
fn multi_amount(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    defaults: Vec<i32>,
    ranges: Vec<(i32, i32)>,
    min: i32,
    max: i32,
) -> Option<Vec<i32>> {
    let n = defaults.len();
    if n == 0 || (n == 1 && min == max) || ranges.iter().all(|(lo, hi)| lo == hi) {
        return Some(defaults);
    }
    let q = Query::MultiAmount { defaults, ranges: ranges.clone(), min, max };
    loop {
        match ask(game, d, seat, &q) {
            Answer::Text(s) => {
                let values: Vec<i32> = s.split_whitespace().map(|v| v.parse().unwrap_or(0)).collect();
                let sum: i32 = values.iter().sum();
                if values.len() == n
                    && values.iter().zip(ranges.iter()).all(|(v, (lo, hi))| v >= lo && v <= hi)
                    && sum >= min
                    && sum <= max
                {
                    return Some(values);
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
    }
}

/// Combat damage (`CombatGroup.assignDamageToBlockers`): each attacker in order assigns its damage among its blockers
/// (asking its controller when there's a choice), then everything is dealt at once. Blocked attackers whose blockers
/// are gone deal no damage, unless they have trample.
fn combat_damage(game: &mut Game, d: &mut dyn Decider, first: bool) {
    let has_first_step = game.first_strike_step;
    let mut hits: Vec<(ObjId, Target, i32)> = Vec::new();
    for a in attackers(game) {
        if !deals_damage_in(game, a, first, has_first_step) {
            continue;
        }
        let (power, _) = game.power_toughness(a);
        let damage = power.max(0);
        let defender = match game.obj(a).attacking {
            Some(s) => s,
            None => continue,
        };
        if !game.obj(a).blocked {
            hits.push((a, Target::Player(defender), damage));
            continue;
        }
        let blockers = live_blockers(game, a);
        let trample = game.has_ability(a, &AbilityDef::TrampleAbility);
        let mut remaining = damage;
        let mut defaults = Vec::new();
        for b in &blockers {
            let n = remaining.min(lethal_damage(game, *b, a));
            remaining -= n;
            defaults.push(n);
        }
        let ranges = vec![(0, damage); blockers.len()];
        let seat = game.obj(a).controller;
        let amounts = if trample {
            if remaining > 0 || blockers.len() > 1 {
                match multi_amount(game, d, seat, defaults, ranges, damage - remaining, damage) {
                    Some(v) => v,
                    None => return,
                }
            } else if blockers.len() == 1 {
                vec![damage]
            } else {
                Vec::new()
            }
        } else {
            if remaining > 0 && !defaults.is_empty() {
                defaults[0] += remaining;
            }
            if blockers.len() > 1 {
                match multi_amount(game, d, seat, defaults, ranges, damage, damage) {
                    Some(v) => v,
                    None => return,
                }
            } else if blockers.len() == 1 {
                vec![damage]
            } else {
                Vec::new()
            }
        };
        for (b, n) in blockers.iter().zip(amounts.iter()) {
            hits.push((a, Target::Obj(*b), *n));
        }
        if trample {
            let over = damage - amounts.iter().sum::<i32>();
            if over > 0 {
                hits.push((a, Target::Player(defender), over));
            }
        }
    }
    for b in Arc::clone(&game.battlefield).iter().copied() {
        let blocking = game.obj(b).blocking.clone();
        if blocking.is_empty() || !deals_damage_in(game, b, first, has_first_step) {
            continue;
        }
        let (power, _) = game.power_toughness(b);
        if let Some(a) = blocking.first() {
            hits.push((b, Target::Obj(*a), power));
        }
    }
    let mut gains: Vec<(ObjId, Seat, i32)> = Vec::new();
    for (src, to, n) in hits {
        let amount = deal_damage(game, src, to, n, true);
        if amount > 0 {
            let seat = game.obj(src).controller;
            match gains.iter_mut().find(|g| g.0 == src) {
                Some(g) => g.2 += amount,
                None => gains.push((src, seat, amount)),
            }
        }
    }
    // combat lifelink: each source's damage is gained at once, in the order the sources dealt it
    for (_, seat, amount) in gains {
        gain_life(game, seat, amount);
    }
}

// ------------------------------------------------------------------------------------------------ cleanup and SBAs

/// Discard to hand size (one card per answer, as `HumanPlayer.choose` asks), then damage wears off.
fn cleanup(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
    let excess = game.player(seat).hand.len().saturating_sub(MAX_HAND);
    if excess > 0 {
        let mut chosen: Vec<ObjId> = Vec::new();
        while chosen.len() < excess {
            let objs: Vec<ObjId> = game.player(seat).hand.iter().copied().filter(|c| !chosen.contains(c)).collect();
            match ask(game, d, seat, &Query::Cards { objs: objs.clone(), required: true }) {
                Answer::Obj(id) if objs.contains(&id) => chosen.push(id),
                Answer::Obj(id) if chosen.contains(&id) => chosen.retain(|c| *c != id),
                Answer::Stop => return,
                _ => {}
            }
        }
        for id in chosen {
            game.move_to(id, Zone::Graveyard);
        }
    }
    if game.boosts.iter().any(|b| b.until_end_of_turn) {
        Arc::make_mut(&mut game.boosts).retain(|b| !b.until_end_of_turn);
    }
    if game.grants.iter().any(|g| g.until_end_of_turn) {
        Arc::make_mut(&mut game.grants).retain(|g| !g.until_end_of_turn);
    }
    game.exile_if_dies.clear();
    for id in Arc::clone(&game.battlefield).iter().copied() {
        if game.obj(id).damage != 0 || game.obj(id).deathtouched {
            let o = game.obj_mut(id);
            o.damage = 0;
            o.deathtouched = false;
        }
    }
}

/// State-based actions, repeated until none apply: players at 0 life or who drew from an empty library lose;
/// creatures with 0 toughness or lethal damage die.
/// The legend rule (704.5j), the last of Java's state-based actions: the first legendary permanent, in battlefield
/// order, whose controller has another with its name; they choose one to keep (`choose`, not targeting) and the rest
/// go to the graveyard. True: one applied, and state-based actions start over.
fn legend_rule(game: &mut Game, d: &mut dyn Decider) -> bool {
    let legendary = |g: &Game, id: ObjId| g.obj(id).def.supertypes.iter().any(|t| t == "LEGENDARY");
    let bf: Vec<ObjId> = game.battlefield.iter().copied().collect();
    for &legend in &bf {
        if !legendary(game, legend) {
            continue;
        }
        let (name, controller) = (game.obj(legend).def.name.clone(), game.obj(legend).controller);
        let same: Vec<ObjId> = bf
            .iter()
            .copied()
            .filter(|id| {
                legendary(game, *id) && game.obj(*id).def.name == name && game.obj(*id).controller == controller
            })
            .collect();
        if same.len() < 2 {
            continue;
        }
        let keep = loop {
            match ask(game, d, controller, &Query::Target { players: Vec::new(), objs: same.clone(), required: true }) {
                Answer::Obj(o) if same.contains(&o) => break o,
                Answer::Stop => return false,
                _ => {}
            }
        };
        for id in same {
            if id != keep {
                game.move_to(id, Zone::Graveyard);
            }
        }
        return true;
    }
    false
}

pub fn state_based_actions(game: &mut Game) {
    loop {
        let mut any = false;
        for s in 0..game.seats() {
            let p = game.player(s);
            if !p.lost && (p.life <= 0 || p.drew_from_empty) {
                let p = game.player_mut(s);
                p.lost = true;
                p.drew_from_empty = false;
                any = true;
            }
        }
        let alive: Vec<Seat> = (0..game.seats()).filter(|s| !game.player(*s).lost).collect();
        if alive.len() <= 1 && !game.over && alive.len() < game.seats() {
            // as Java's trace sees the end: the loser's cards are gone from hand, library and graveyard (their
            // permanents stay), and nobody is marked as the winner yet
            for s in 0..game.seats() {
                if game.player(s).lost {
                    let p = game.player_mut(s);
                    p.hand.clear();
                    p.library.clear();
                    p.graveyard.clear();
                }
            }
            game.over = true;
            return;
        }
        let dying: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                let o = game.obj(*id);
                if !o.is(CardType::Creature) {
                    return false;
                }
                let (_, t) = game.power_toughness(*id);
                t <= 0
                    || (!game.has_ability(*id, &AbilityDef::IndestructibleAbility)
                        && (o.damage >= t || (o.deathtouched && o.damage > 0)))
            })
            .collect();
        for id in dying {
            game.move_to(id, Zone::Graveyard);
            any = true;
        }
        if !any {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decide::Passer;

    fn game_with(db: &crate::card::CardDb, deck: &[(&str, usize)]) -> Game {
        let mut game = Game::new(2, 42);
        for seat in 0..2 {
            for (name, n) in deck {
                for _ in 0..*n {
                    let id = game.create_obj(db.get(name).unwrap(), seat, Zone::Library);
                    game.player_mut(seat).library.push(id);
                }
            }
            draw(&mut game, seat, 7);
        }
        game.turn = 1;
        game.step = Some(Step::Upkeep);
        game.active = Some(0);
        game
    }

    #[test]
    fn passers_play_to_the_turn_limit() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 24), ("Bear Cub", 36)]);
        play_from_priority(&mut game, &mut Passer, 20);
        assert_eq!(game.turn, 19);
        assert!(!game.over);
        assert_eq!(game.player(0).hand.len(), 7);
    }

    #[test]
    fn shock_resolves_and_kills() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Mountain", 10)]);
        let bear = game.create_obj(db.get("Bear Cub").unwrap(), 1, Zone::Hand);
        game.move_to(bear, Zone::Battlefield);
        let shock = game.create_obj(db.get("Shock").unwrap(), 0, Zone::Stack);
        let zcc = game.obj(bear).zcc;
        Arc::make_mut(&mut game.stack).push(StackItem {
            targets: vec![vec![Target::Obj(bear)]],
            target_zcc: vec![vec![zcc]],
            ..StackItem::spell(shock, 0)
        });
        resolve_top(&mut game, &mut Passer);
        state_based_actions(&mut game);
        assert_eq!(game.player(1).graveyard, vec![bear]);
        assert_eq!(game.player(0).graveyard, vec![shock]);
    }

    #[test]
    fn bite_down_uses_the_biters_deathtouch() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let spider = game.create_obj(db.get("Treetop Snarespinner").unwrap(), 0, Zone::Hand);
        game.move_to(spider, Zone::Battlefield);
        let sentry = game.create_obj(db.get("Magnigoth Sentry").unwrap(), 1, Zone::Hand);
        game.move_to(sentry, Zone::Battlefield);
        let bite = game.create_obj(db.get("Bite Down").unwrap(), 0, Zone::Stack);
        let zcc = (game.obj(spider).zcc, game.obj(sentry).zcc);
        Arc::make_mut(&mut game.stack).push(StackItem {
            targets: vec![vec![Target::Obj(spider)], vec![Target::Obj(sentry)]],
            target_zcc: vec![vec![zcc.0], vec![zcc.1]],
            ..StackItem::spell(bite, 0)
        });
        resolve_top(&mut game, &mut Passer);
        state_based_actions(&mut game);
        assert_eq!(game.player(1).graveyard, vec![sentry]);
        assert_eq!(game.obj(spider).damage, 0);
    }

    #[test]
    fn kicked_colony_enters_with_counters_and_tramples() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let colony = game.create_obj(db.get("Gnarlid Colony").unwrap(), 0, Zone::Stack);
        Arc::make_mut(&mut game.stack).push(StackItem { kicked: true, ..StackItem::spell(colony, 0) });
        let bear = game.create_obj(db.get("Bear Cub").unwrap(), 0, Zone::Hand);
        game.move_to(bear, Zone::Battlefield);
        resolve_top(&mut game, &mut Passer);
        assert_eq!(game.power_toughness(colony), (4, 4));
        assert!(game.has_ability(colony, &AbilityDef::TrampleAbility));
        assert!(!game.has_ability(bear, &AbilityDef::TrampleAbility));
    }
}
