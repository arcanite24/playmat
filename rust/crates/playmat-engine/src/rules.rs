//! The rules: turns and steps, priority, playing lands and casting spells, paying mana, resolving, combat,
//! state-based actions. The order of everything a player sees follows Java's `GameImpl.playPriority` and
//! `HumanPlayer` with the client's default settings, so a shadow game replays answer for answer.

use crate::card::{
    AbilityDef, Amount, CardType, Cmp, Color, Condition, CostDef, Duration, EffectDef, Mana, ManaCost, ManaKind, Pred, TargetDef,
    Trigger, CardDef,
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
                p.life_lost = 0;
                p.drawn = 0;
                p.noncreature_cast = 0;
                p.permanent_left = false;
                p.life_gains = 0;
                p.spells_cast = 0;
                p.instants_sorceries_cast = 0;
            }
            game.triggered_this_turn.clear();
            game.activated_this_turn.clear();
            for id in Arc::clone(&game.battlefield).iter().copied() {
                if game.obj(id).controller == active {
                    let o = game.obj_mut(id);
                    untap(o);
                    o.sick = false;
                }
            }
        }
        Step::DeclareAttackers => {
            declare_attackers(game, d, active);
            if !attackers(game).is_empty() {
                game.player_mut(active).attacked = true;
            }
            // "whenever this attacks" triggers, in the order the attackers are listed
            for a in attackers(game) {
                let sources: Vec<ObjId> = game.trigger_sources();
                let fired = game.triggers(sources, |g, src, you, t| match t {
                    Trigger::Attacks | Trigger::EntersOrAttacks => src == a,
                    Trigger::AttacksAll { filter } => g.filter_matches(filter, you, Some(src), a),
                    _ => false,
                });
                for mut t in fired {
                    let all = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities[i].trigger().cloned());
                    if matches!(all, Some(Trigger::AttacksAll { .. })) && set_target_pointer(game, &t) {
                        t.targets = vec![vec![Target::Obj(a)]];
                        t.target_zcc = vec![vec![game.obj(a).zcc]];
                    }
                    game.pending.push(t);
                }
            }
            // then DECLARED_ATTACKERS: "whenever you attack with N or more"
            let all = attackers(game);
            if !all.is_empty() {
                let sources: Vec<ObjId> = game.trigger_sources();
                let fired = game.triggers(sources, |g, src, you, t| {
                    matches!(t, Trigger::AttacksWith { min, filter }
                        if you == active
                            && all.iter().filter(|a| g.filter_matches(filter, you, Some(src), **a)).count() >= *min)
                });
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
        Step::Upkeep | Step::Draw | Step::BeginCombat | Step::PostcombatMain | Step::EndTurn => {
            // the draw step's draw comes before its triggers
            if step == Step::Draw {
                draw(game, active, 1);
            }
            let name = match step {
                Step::Upkeep => "upkeep",
                Step::Draw => "draw",
                Step::BeginCombat => "combat",
                Step::PostcombatMain => "main2",
                _ => "end",
            };
            let mut sources: Vec<ObjId> = game.trigger_sources();
            for p in game.players.iter() {
                sources.extend(p.graveyard.iter().copied());
            }
            let fired = game.triggers(sources, |g, src, you, t| {
                matches!(t, Trigger::StepBegins { step, who, graveyard }
                    if step == name && Game::whose(who, you, active)
                        && *graveyard == (g.obj(src).zone == Zone::Graveyard))
            });
            game.pending.extend(fired);
            // "at the beginning of the next end step" delayed triggers, after the others
            if step == Step::EndTurn {
                let delayed = std::mem::take(&mut game.at_next_end_step);
                game.pending.extend(delayed);
            }
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
                o.attacking_pw = None;
                o.blocking.clear();
                o.blocked = false;
                o.blocked_by.clear();
            }
        }
    }
}

/// Draws `n` cards, one at a time: each card drawn is a `DREW_CARD` event for "whenever … draws" triggers.
pub fn draw(game: &mut Game, seat: Seat, n: usize) {
    for _ in 0..n {
        // Narset, Parter of Veils: each opponent can't draw more than one card each turn
        if game.player(seat).drawn >= 1
            && game.any_static(|g, src, e| {
                matches!(e, EffectDef::NarsetParterOfVeilsEffect) && g.obj(src).controller != seat
            })
        {
            continue;
        }
        match game.player(seat).library.first().copied() {
            Some(id) => {
                game.player_mut(seat).drawn += 1;
                game.move_to(id, Zone::Hand);
                let sources: Vec<ObjId> = game.trigger_sources();
                let fired = game
                    .triggers(sources, |_, _, you, t| matches!(t, Trigger::Draws { who } if Game::whose(who, you, seat)));
                for mut t in fired {
                    if set_target_pointer(game, &t) {
                        t.targets = vec![vec![Target::Player(seat)]];
                        t.target_zcc = vec![vec![0]];
                    }
                    game.pending.push(t);
                }
            }
            None => game.player_mut(seat).drew_from_empty = true,
        }
    }
}

/// Whether a waiting trigger's ability points its effects at what the event was about.
fn set_target_pointer(game: &Game, t: &StackItem) -> bool {
    let a = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i));
    matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. }))
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
            if can_play_land(game, seat) || !activatable(game, seat, *id).is_empty() {
                out.push(*id);
            }
        } else if castable(game, seat, *id, &o.def)
            || o.def.adventure.as_ref().is_some_and(|a| castable(game, seat, *id, a))
            || spectacle(game, seat, *id).is_some()
            || warp(game, seat, *id).is_some()
            || !activatable(game, seat, *id).is_empty()
        {
            out.push(*id);
        }
    }
    // flashback, from the graveyard
    for id in &p.graveyard {
        if game.obj(*id).def.flashback().is_some_and(|f| castable(game, seat, *id, &f)) {
            out.push(*id);
        }
    }
    // a card on an adventure: its creature, from exile; a card you may play from exile
    for id in game.exile.iter() {
        let o = game.obj(*id);
        if o.on_adventure && o.owner == seat && castable(game, seat, *id, &o.def) {
            out.push(*id);
        } else if (o.may_play.is_some_and(|(p, t)| p == seat && game.turn <= t)
            || o.warp_from.is_some_and(|t| o.owner == seat && game.turn >= t))
            && if o.is(CardType::Land) { can_play_land(game, seat) } else { castable(game, seat, *id, &o.def) }
        {
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
    if activations_forbidden(game, id) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, a) in o.def.abilities.iter().enumerate() {
        let Some((_, _, targets, costs, sorcery)) = a.activated() else { continue };
        // cycling works from hand, everything else on the battlefield
        if a.from_hand() != (o.zone == Zone::Hand) {
            continue;
        }
        if sorcery && !sorcery_speed(game, seat) {
            continue;
        }
        if let AbilityDef::LimitedTimesPerTurnActivatedAbility { limit, condition, .. } = a {
            let used = game.activated_this_turn.iter().filter(|x| **x == (id, i)).count() as u32;
            if used >= *limit || !game.condition_holds(condition.as_ref(), seat, id) {
                continue;
            }
        }
        let payable = costs.iter().all(|c| match c {
            CostDef::ManaCost { .. } => can_pay(game, seat, &adjusted_mana_cost(game, seat, a, costs)),
            _ => can_pay_cost(game, seat, id, c),
        });
        let targeted = targets.iter().all(|t| target_possible(game, seat, t));
        if payable && targeted {
            out.push(i);
        }
    }
    out
}

/// Whether a cost of `id`'s ability (or spell) can be paid now (`Cost.canPay`).
fn can_pay_cost(game: &Game, seat: Seat, id: ObjId, c: &CostDef) -> bool {
    let o = game.obj(id);
    match c {
        CostDef::ManaCost { cost } => ManaCost::parse(cost).map(|m| can_pay(game, seat, &m)).unwrap_or(false),
        CostDef::TapSourceCost => {
            let sick = game.has_type(id, CardType::Creature) && o.sick && !game.has_ability(id, &AbilityDef::HasteAbility);
            !(o.tapped || sick)
        }
        CostDef::SacrificeSourceCost => true,
        CostDef::PayLoyaltyCost { amount } => !o.loyalty_used && o.counter("loyalty") + amount >= 0,
        // you can only sacrifice your own permanents
        CostDef::SacrificeTargetCost { filter } => {
            matching(game, filter, seat, id).iter().any(|p| game.obj(*p).controller == seat)
        }
        CostDef::TapTargetCost { filter, amount } => {
            matching(game, filter, seat, id).iter().filter(|p| game.obj(**p).controller == seat).count() >= *amount
        }
        CostDef::CrewCost { value } => {
            crew_candidates(game, seat, id).iter().map(|c| game.power_toughness(*c).0.max(0)).sum::<i32>() >= *value
        }
        CostDef::DiscardCardCost { filter } => {
            game.player(seat).hand.iter().any(|c| game.filter_matches(filter, seat, Some(id), *c))
        }
        CostDef::PayLifeCost { amount } => game.player(seat).life >= *amount,
        CostDef::DiscardSourceCost => o.zone == Zone::Hand,
        // either one (`OrCost`)
        CostDef::OrCost { costs } => costs.iter().any(|c| or_cost_payable(game, seat, id, c)),
    }
}

/// An `OrCost` part's `canPay`: a mana cost's is always true.
fn or_cost_payable(game: &Game, seat: Seat, id: ObjId, c: &CostDef) -> bool {
    matches!(c, CostDef::ManaCost { .. }) || can_pay_cost(game, seat, id, c)
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

/// Whether the card (or its adventure, `def`) can be cast now: timing, the one-spell limit, its costs, its targets.
fn castable(game: &Game, seat: Seat, id: ObjId, def: &CardDef) -> bool {
    (def.is(CardType::Instant) || def.has(&AbilityDef::FlashAbility) || sorcery_speed(game, seat) || {
        game.any_static(|g, src, e| match e {
            EffectDef::CastAsThoughItHadFlashAllEffect { filter } => {
                g.obj(src).controller == seat && g.filter_matches(filter, seat, Some(src), id)
            }
            _ => false,
        })
    })
        && !cant_cast_more(game, seat)
        && can_pay(game, seat, &modified_cost(game, seat, id, def.cost.clone()))
        && def.spell.costs.iter().all(|c| can_pay_cost(game, seat, id, c))
        && has_targets(game, seat, def)
}

/// Warp: the card's ability index and the card with its warp cost, if it can be cast that way from hand (at sorcery
/// speed only).
fn warp(game: &Game, seat: Seat, id: ObjId) -> Option<(usize, CardDef)> {
    if game.obj(id).zone != Zone::Hand || !sorcery_speed(game, seat) {
        return None;
    }
    let (k, def) = game.obj(id).def.warp()?;
    castable(game, seat, id, &def).then_some((k, def))
}

/// Spectacle: the card's ability index and the card with its spectacle cost, if an opponent lost life this turn and
/// it can be cast that way.
fn spectacle(game: &Game, seat: Seat, id: ObjId) -> Option<(usize, CardDef)> {
    let (k, def) = game.obj(id).def.spectacle()?;
    (game.condition_holds(Some(&Condition::OpponentsLostLife), seat, id) && castable(game, seat, id, &def))
        .then_some((k, def))
}

/// Whether the spell has its targets: for a modal spell, whether some mode does.
fn has_targets(game: &Game, seat: Seat, def: &CardDef) -> bool {
    let spell = &def.spell;
    let all = |targets: &[TargetDef]| targets.iter().all(|t| target_possible(game, seat, t));
    if spell.modes.is_empty() {
        all(&spell.targets)
    } else {
        spell.modes.iter().any(|m| all(&m.targets))
    }
}

/// Untapped permanents that can make mana now, each with its mana abilities (index, mana), in ability order.
fn mana_sources(game: &Game, seat: Seat) -> Vec<(ObjId, Vec<(usize, Mana)>)> {
    mana_sources_with_kinds(game, seat)
        .into_iter()
        .map(|(id, a)| (id, a.into_iter().map(|(i, m, _)| (i, m)).collect()))
        .collect()
}

/// Clarion Conqueror: the activated abilities (mana abilities too) of artifacts, creatures and planeswalkers on the
/// battlefield can't be activated.
fn activations_forbidden(game: &Game, id: ObjId) -> bool {
    game.obj(id).zone == Zone::Battlefield
        && [CardType::Artifact, CardType::Creature, CardType::Planeswalker].iter().any(|t| game.has_type(id, *t))
        && game.any_static(|_, _, e| matches!(e, EffectDef::ClarionConquerorEffect))
}

/// Untapped permanents with a mana ability their controller can activate, and those abilities.
fn mana_sources_with_kinds(game: &Game, seat: Seat) -> Vec<(ObjId, Vec<(usize, Mana, ManaKind)>)> {
    let mut out = Vec::new();
    for id in game.battlefield.iter() {
        let o = game.obj(*id);
        if o.controller != seat || o.tapped || activations_forbidden(game, *id) {
            continue;
        }
        if game.has_type(*id, CardType::Creature) && o.sick && !game.has_ability(*id, &AbilityDef::HasteAbility) {
            continue;
        }
        let abilities: Vec<(usize, Mana, ManaKind)> = o
            .def
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, a)| a.mana_condition().is_none_or(|c| game.condition_holds(Some(c), seat, *id)))
            .filter(|(_, a)| a.mana_costs().iter().all(|c| match c {
                // `canPay` counts the source itself, still untapped
                CostDef::TapTargetCost { filter, amount } => {
                    mana_tap_targets(game, seat, *id, filter).len() + usize::from(game.filter_matches(filter, seat, Some(*id), *id))
                        >= *amount
                }
                CostDef::PayLifeCost { amount } => game.player(seat).life >= *amount,
                _ => true,
            }))
            .filter_map(|(i, a)| a.mana().map(|m| (i, m, a.mana_kind())))
            .collect();
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

/// `ManaUtil.tryToAutoPay`: with only basic and any-color abilities, the one basic ability whose color is still
/// owed; with no color owed and generic left, the first (the first that only taps, when nothing colored is owed at
/// all); otherwise all of them (the player picks).
fn auto_pay_abilities(abilities: &[(usize, Mana, ManaKind)], unpaid: &ManaCost) -> Vec<(usize, Mana, ManaKind)> {
    if abilities.iter().any(|a| a.2 == ManaKind::Other) {
        return abilities.to_vec();
    }
    let owed: Vec<&(usize, Mana, ManaKind)> = abilities
        .iter()
        .filter(|(_, m, k)| {
            *k == ManaKind::Basic && Color::ALL.iter().any(|c| m.colored[c.index()] > 0 && unpaid.colored[c.index()] > 0)
        })
        .collect();
    let colors: std::collections::BTreeSet<usize> = owed
        .iter()
        .flat_map(|(_, m, _)| Color::ALL.iter().filter(|c| m.colored[c.index()] > 0).map(|c| c.index()))
        .collect();
    match colors.len() {
        0 if unpaid.generic > 0 => {
            let no_colors = unpaid.colored.iter().all(|n| *n == 0) && unpaid.colorless == 0;
            let plain = abilities.iter().find(|a| a.2 == ManaKind::Basic).filter(|_| no_colors);
            vec![*plain.unwrap_or(&abilities[0])]
        }
        1 => vec![**owed.last().unwrap()],
        _ => abilities.to_vec(),
    }
}

/// `ManaUtil.tryToAutoSelectAManaColor`: the one color still owed; with none owed, white; else the player chooses
/// (CHOOSE_CHOICE). None: stopped.
fn any_color(game: &mut Game, d: &mut dyn Decider, seat: Seat, unpaid: &ManaCost) -> Option<Color> {
    let owed: Vec<Color> = Color::ALL.iter().copied().filter(|c| unpaid.colored[c.index()] > 0).collect();
    match owed.len() {
        0 => return Some(Color::W),
        1 => return Some(owed[0]),
        _ => {}
    }
    let names = ["White", "Blue", "Black", "Red", "Green"];
    let choices: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    let q = Query::Choice { message: "Select a color of mana to add 1 of it".into(), choices };
    loop {
        match ask(game, d, seat, &q) {
            Answer::Text(t) => {
                if let Some(c) = names.iter().position(|n| *n == t).map(|i| [Color::W, Color::U, Color::B, Color::R, Color::G][i])
                {
                    return Some(c);
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}

fn play(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId) {
    if game.obj(id).zone == Zone::Battlefield {
        let activatable = activatable(game, seat, id);
        // `getPlayableActivatedAbilities` offers its mana abilities too
        let mana: Vec<(usize, Mana, ManaKind)> =
            mana_sources_with_kinds(game, seat).into_iter().find(|(s, _)| *s == id).map(|(_, a)| a).unwrap_or_default();
        let mut abilities = activatable.clone();
        abilities.extend(mana.iter().map(|(i, _, _)| *i));
        abilities.sort();
        // `HumanPlayer.activateAbility`: one ability with mana in its cost, or a mana ability, activates directly;
        // otherwise the picker
        let direct = abilities.len() == 1
            && (mana.iter().any(|(i, _, _)| *i == abilities[0]) || {
                let (_, _, _, costs, _) = game.obj(id).def.abilities[abilities[0]].activated().unwrap();
                ability_mana_cost(costs).mana_value() > 0
            });
        let index = if direct {
            abilities[0]
        } else {
            match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: abilities.clone() }) {
                Answer::Ability(o, i) if o == id && abilities.contains(&i) => i,
                _ => return,
            }
        };
        if let Some((i, m, k)) = mana.iter().find(|(i, _, _)| *i == index).copied() {
            // `PlayerImpl.activateAbility`: an action taken resets everyone's passed flag
            if activate_mana(game, d, seat, id, i, m, k, &ManaCost::default()) {
                for s in 0..game.seats() {
                    game.player_mut(s).passed = false;
                }
            }
            return;
        }
        let before = game.clone();
        if !activate(game, d, seat, id, index) {
            *game = before;
        }
        return;
    }
    // a card in hand with an ability (cycling, channel): the picker, with its spell or land play (a0) and the abilities, if there is a choice
    if game.obj(id).zone == Zone::Hand && game.obj(id).def.adventure.is_none() {
        let mut options = activatable(game, seat, id);
        let spectacle = spectacle(game, seat, id);
        if let Some((k, _)) = &spectacle {
            options.push(*k);
        }
        let warp = warp(game, seat, id);
        if let Some((k, _)) = &warp {
            options.push(*k);
        }
        if !options.is_empty() {
            let def = game.obj(id).def.clone();
            if if def.is(CardType::Land) { can_play_land(game, seat) } else { castable(game, seat, id, &def) } {
                options.insert(0, 0);
            }
            let index = if options.len() == 1 {
                options[0]
            } else {
                match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: options.clone() }) {
                    Answer::Ability(o, i) if o == id && options.contains(&i) => i,
                    _ => return,
                }
            };
            // cast for its spectacle or warp cost (the whole card comes back as it leaves the stack)
            let alternative = spectacle.filter(|(k, _)| *k == index).map(|(_, def)| (def, false));
            if let Some((def, warped)) = alternative.or(warp.filter(|(k, _)| *k == index).map(|(_, def)| (def, true))) {
                let before = game.clone();
                let full = game.obj(id).def.clone();
                let o = game.obj_mut(id);
                o.adventurer = Some(full);
                o.def = Arc::new(def);
                if !cast(game, d, seat, id) {
                    *game = before;
                    return;
                }
                game.obj_mut(id).warped = warped;
                return;
            }
            if index != 0 {
                let before = game.clone();
                if !activate(game, d, seat, id, index) {
                    *game = before;
                }
                return;
            }
        }
    }
    if game.obj(id).is(CardType::Land) {
        // a modal double-faced land: the picker, with its front's and its back's play abilities (the back's come
        // after the front's)
        if let Some(back) = game.obj(id).def.back.clone() {
            let b = game.obj(id).def.abilities.len();
            match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: vec![0, b] }) {
                Answer::Ability(o, 0) if o == id => {}
                Answer::Ability(o, i) if o == id && i == b => {
                    let full = game.obj(id).def.clone();
                    let o = game.obj_mut(id);
                    o.front = Some(full);
                    o.def = back;
                }
                _ => return,
            }
        }
        enter_battlefield(game, d, id, seat, false);
        game.player_mut(seat).lands_played += 1;
        return;
    }
    let before = game.clone();
    // flashback: cast with its flashback cost (the whole card comes back as it leaves the stack)
    if game.obj(id).zone == Zone::Graveyard {
        let Some(f) = game.obj(id).def.flashback() else { return };
        let full = game.obj(id).def.clone();
        let o = game.obj_mut(id);
        o.adventurer = Some(full);
        o.def = Arc::new(f);
        if !cast(game, d, seat, id) {
            *game = before;
            return;
        }
        game.obj_mut(id).flashback = true;
        return;
    }
    // an adventurer card: the picker, with the adventure (a0) and the creature (a1) that can be cast
    if let Some(adventure) = game.obj(id).def.adventure.clone() {
        let o = game.obj(id);
        let mut options = Vec::new();
        if o.zone == Zone::Hand && castable(game, seat, id, &adventure) {
            options.push(0);
        }
        if castable(game, seat, id, &o.def.clone()) {
            options.push(1);
        }
        match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: options.clone() }) {
            Answer::Ability(o, 0) if o == id && options.contains(&0) => {
                let full = game.obj(id).def.clone();
                let o = game.obj_mut(id);
                o.adventurer = Some(full);
                o.def = adventure;
            }
            Answer::Ability(o, 1) if o == id && options.contains(&1) => {}
            _ => return,
        }
    }
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
        o.on_adventure = false;
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
    // X: `announceX` (AMOUNT) asks until an answer is in range; X's mana is then generic
    if cost.x > 0 {
        let q = Query::Amount { min: 0, max: i32::MAX };
        let x = loop {
            match ask(game, d, seat, &q) {
                Answer::Int(n) if n >= 0 => break n,
                Answer::Stop => return false,
                _ => {}
            }
        };
        cost.generic += x as u32 * cost.x;
        cost.x = 0;
        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
            top.x = x;
        }
    }
    // `Modes.choose` / `HumanPlayer.chooseMode`: a mode per question until the most are chosen; any other answer ends
    // the choice (Cancel clears it), and fewer than the least cancels the cast
    let mut modes: Vec<usize> = Vec::new();
    let n = def.spell.modes.len();
    if n > 1 {
        let (min, max) = (def.spell.min_modes, def.spell.max_modes);
        let possible = |game: &Game, k: usize| def.spell.modes[k].targets.iter().all(|t| target_possible(game, seat, t));
        if n == min {
            modes = (0..n).filter(|k| possible(game, *k)).collect();
        } else {
            while modes.len() < max {
                let available: Vec<usize> = (0..n).filter(|k| !modes.contains(k) && possible(game, *k)).collect();
                if available.is_empty() {
                    break;
                }
                match ask(game, d, seat, &Query::ChooseMode { modes: available.clone() }) {
                    Answer::Mode(k) if available.contains(&k) => modes.push(k),
                    Answer::ModeDone if modes.len() >= min => break,
                    Answer::Stop => return false,
                    _ => {
                        modes.clear();
                        break;
                    }
                }
            }
        }
        if modes.len() < min {
            return false;
        }
        modes.sort();
        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
            top.modes = modes.clone();
        }
    }
    let mut defs: Vec<TargetDef> = Vec::new();
    for k in if modes.is_empty() { vec![0] } else { modes.clone() } {
        defs.extend(def.spell.mode(k).1.iter().cloned());
    }
    if let Some((unkicked, kicked)) = &def.spell.kicked_targets {
        let k = game.stack.last().is_some_and(|t| t.kicked);
        defs = if k { kicked.clone() } else { unkicked.clone() };
    }
    if !choose_targets(game, d, seat, &defs, Some(id), &def.spell.rule, true) {
        return false;
    }
    for s in 0..game.seats() {
        game.player_mut(s).passed = false;
    }
    let cost = modified_cost(game, seat, id, cost);
    if !pay_mana(game, d, seat, &cost) {
        return false;
    }
    // 601.2g: then the other costs
    for c in &def.spell.costs {
        if !pay_cost(game, d, seat, id, c) {
            return false;
        }
    }
    // SPELL_CAST
    let spell = id;
    {
        let instant_or_sorcery = def.is(CardType::Instant) || def.is(CardType::Sorcery);
        let p = game.player_mut(seat);
        p.spells_cast += 1;
        if !def.is(CardType::Creature) {
            p.noncreature_cast += 1;
        }
        if instant_or_sorcery {
            p.instants_sorceries_cast += 1;
        }
    }
    let sources: Vec<ObjId> = game.trigger_sources();
    let targeted: Vec<ObjId> = game
        .stack
        .last()
        .map(|t| t.targets.iter().flatten().filter_map(|x| if let Target::Obj(o) = x { Some(*o) } else { None }).collect())
        .unwrap_or_default();
    let fired = game.triggers(sources.clone(), |g, src, you, t| match t {
        Trigger::SpellCast { filter, who } => Game::whose(who, you, seat) && g.filter_matches(filter, you, Some(src), spell),
        Trigger::SpellCastTargetingThis => you == seat && targeted.contains(&src),
        Trigger::CastSecondSpell { who } => Game::whose(who, you, seat) && g.player(seat).spells_cast == 2,
        _ => false,
    });
    let mut fired = fired;
    for t in fired.iter_mut() {
        let a = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i));
        if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. })) {
            t.targets = vec![vec![Target::Player(seat)]];
            t.target_zcc = vec![vec![0]];
        }
    }
    game.pending.extend(fired);
    // TARGETED, a simultaneous event: handled after SPELL_CAST
    let mut fired = game.triggers(sources, |g, src, you, t| match t {
        Trigger::BecomesTargetOfSpell { filter } => targeted.contains(&src) && g.filter_matches(filter, you, Some(src), spell),
        _ => false,
    });
    for t in fired.iter_mut() {
        if set_target_pointer(game, t) {
            t.targets = vec![vec![Target::Player(seat)]];
            t.target_zcc = vec![vec![0]];
        }
    }
    game.pending.extend(fired);
    ward(game);
    true
}

/// `CantCastMoreThanOneSpellEffect`: the player already cast a spell this turn.
fn cant_cast_more(game: &Game, seat: Seat) -> bool {
    game.player(seat).spells_cast >= 1
        && game.any_static(|g, src, e| match e {
            EffectDef::CantCastMoreThanOneSpellEffect { who } => Game::whose(who, g.obj(src).controller, seat),
            _ => false,
        })
}

/// `ContinuousEffects.costModification`: a spell's cost with the increases, then the reductions, applied.
fn modified_cost(game: &Game, seat: Seat, id: ObjId, mut cost: ManaCost) -> ManaCost {
    for src in game.battlefield.iter() {
        let s = game.obj(*src);
        for a in &s.def.abilities {
            if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                for e in effects {
                    if let EffectDef::SpellsCostIncreasingAllEffect { amount, filter, who } = e {
                        if Game::whose(who, s.controller, seat) && game.filter_matches(filter, s.controller, Some(*src), id) {
                            cost.generic += *amount as u32;
                        }
                    }
                }
            }
        }
    }
    let o = game.obj(id);
    for a in &o.def.abilities {
        if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
            for e in effects {
                if let EffectDef::SpellCostReductionSourceEffect { amount, condition, stack_only } = e {
                    // a condition that only works on the stack is skipped while checking what's playable
                    // (`canWorksOnStackOnly`): the card can go on the stack anyway
                    if (*stack_only && o.zone != Zone::Stack) || game.condition_holds(condition.as_ref(), seat, id) {
                        cost.generic = cost.generic.saturating_sub(*amount as u32);
                    }
                }
                if let EffectDef::SpellCostReductionForEachSourceEffect { per, amount } = e {
                    let n = game.value(amount, seat, id, 0) * per;
                    cost.generic = cost.generic.saturating_sub(n.max(0) as u32);
                }
            }
        }
    }
    cost
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
    // TARGETED for an ability (a spell's, after SPELL_CAST, in `cast`)
    if game.stack.last().is_some_and(|t| t.ability.is_some()) {
        ward(game);
    }
    true
}

/// `WardAbility`: the permanents the top of the stack targets whose controller is that object's opponent, each with
/// a ward trigger countering it
fn ward(game: &mut Game) {
    let Some(top) = game.stack.last().cloned() else { return };
    // valiant: the first time each turn a permanent becomes a target (`NumberOfTimesPermanentTargetedATurnWatcher`)
    let mut first = Vec::new();
    for t in top.targets.iter().flatten() {
        let Target::Obj(o) = *t else { continue };
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let turn = game.turn;
        let o_ = game.obj_mut(o);
        o_.targeted = if o_.targeted.0 == turn { (turn, o_.targeted.1 + 1) } else { (turn, 1) };
        if o_.targeted.1 == 1 && o_.controller == top.controller {
            first.push(o);
        }
    }
    let fired = game.triggers(first.clone(), |_, src, _, t| matches!(t, Trigger::Valiant) && first.contains(&src));
    game.pending.extend(fired);
    let mut fired = Vec::new();
    for t in top.targets.iter().flatten() {
        let Target::Obj(o) = *t else { continue };
        let obj = game.obj(o);
        if obj.zone != Zone::Battlefield || obj.controller == top.controller {
            continue;
        }
        for (i, a) in obj.def.abilities.iter().enumerate() {
            if let AbilityDef::WardAbility { .. } = a {
                let mut item = StackItem { ability: Some((i, obj.zcc)), ..StackItem::spell(o, obj.controller) };
                match top.ability {
                    Some(ability) => item.ward_of = Some((top.card, ability)),
                    None => {
                        item.targets = vec![vec![Target::Obj(top.card)]];
                        item.target_zcc = vec![vec![game.obj(top.card).zcc]];
                    }
                }
                fired.push(item);
            }
        }
    }
    game.pending.extend(fired);
}
/// `PlayerImpl.playAbility`: the ability goes on the stack, then its targets, then mana, then other costs.
/// False: cancelled.
fn activate(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, index: usize) -> bool {
    let def = game.obj(id).def.clone();
    let Some((rule, _, targets, costs, _)) = def.abilities[index].activated() else { return false };
    let zcc = game.obj(id).zcc;
    if let AbilityDef::LimitedTimesPerTurnActivatedAbility { .. } = def.abilities[index] {
        game.activated_this_turn.push((id, index));
    }
    Arc::make_mut(&mut game.stack).push(StackItem { ability: Some((index, zcc)), ..StackItem::spell(id, seat) });
    if !choose_targets(game, d, seat, targets, None, rule, true) {
        return false;
    }
    for s in 0..game.seats() {
        game.player_mut(s).passed = false;
    }
    if !pay_mana(game, d, seat, &adjusted_mana_cost(game, seat, &def.abilities[index], costs)) {
        return false;
    }
    for c in costs {
        if !matches!(c, CostDef::ManaCost { .. }) && !pay_cost(game, d, seat, id, c) {
            return false;
        }
    }
    true
}

/// `CounterUnlessPaysEffect`'s payment: `canPay`, `chooseUse` "Pay {2}?" (a mana cost's `canPay` is always true), then
/// the payment; anything short of paying counters. None: stopped.
fn pay_unless(game: &mut Game, d: &mut dyn Decider, payer: Seat, cost: &[CostDef]) -> Option<bool> {
    let life: i32 = cost.iter().map(|c| if let CostDef::PayLifeCost { amount } = c { *amount } else { 0 }).sum();
    let mana = ability_mana_cost(cost);
    if game.player(payer).life < life {
        return Some(false);
    }
    let message = if life > 0 { format!("Pay {} life?", life) } else { format!("Pay {}?", mana.text) };
    let q = Query::YesNo { message };
    let yes = loop {
        match ask(game, d, payer, &q) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
    };
    let paid = yes
        && if life > 0 {
            lose_life(game, payer, life);
            true
        } else {
            pay_mana(game, d, payer, &mana)
        };
    if d.stopped() {
        return None;
    }
    Some(paid)
}

/// An activated ability's mana cost, after its cost adjuster (Channel's legendary discount)
fn adjusted_mana_cost(game: &Game, seat: Seat, a: &AbilityDef, costs: &[CostDef]) -> ManaCost {
    let mut cost = ability_mana_cost(costs);
    if let AbilityDef::ChannelAbility { legendary_discount: true, .. } = a {
        let legends = game
            .battlefield
            .iter()
            .filter(|b| {
                let o = game.obj(**b);
                o.controller == seat && o.is(CardType::Creature) && o.def.supertypes.iter().any(|s| s == "LEGENDARY")
            })
            .count();
        cost.generic = cost.generic.saturating_sub(legends as u32);
    }
    cost
}

/// Pays one non-mana cost of `id`'s ability or spell (an `OrCost`'s mana cost too). False: cancelled or stopped.
fn pay_cost(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, c: &CostDef) -> bool {
    match c {
        // tapped already (it paid mana for this ability): the activation fails and is rolled back
        CostDef::TapSourceCost => {
            if game.obj(id).tapped {
                return false;
            }
            game.obj_mut(id).tapped = true
        }
        CostDef::SacrificeSourceCost => {
            let power = game.power_toughness(id).0;
            if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                top.lki_power = Some(power);
            }
            sacrifice(game, id)
        }
        CostDef::PayLoyaltyCost { amount } => {
            let o = game.obj_mut(id);
            o.add_counters("loyalty", *amount);
            o.loyalty_used = true;
        }
        CostDef::TapTargetCost { filter, amount } => {
            let possible: Vec<ObjId> =
                matching(game, filter, seat, id).into_iter().filter(|p| game.obj(*p).controller == seat).collect();
            let Some(chosen) = choose_cards(game, d, seat, &possible, &possible, *amount, *amount, "tap") else {
                return false;
            };
            for o in chosen {
                game.obj_mut(o).tapped = true;
            }
        }
        CostDef::CrewCost { value } => {
            let possible = crew_candidates(game, seat, id);
            let Some(chosen) = choose_cards(game, d, seat, &possible, &possible, 0, usize::MAX, "tap") else {
                return false;
            };
            let mut power = 0;
            for o in chosen {
                if !game.obj(o).tapped {
                    game.obj_mut(o).tapped = true;
                    power += game.power_toughness(o).0;
                }
            }
            if power < *value {
                return false;
            }
        }
        CostDef::SacrificeTargetCost { filter } => {
            let possible: Vec<ObjId> =
                matching(game, filter, seat, id).into_iter().filter(|p| game.obj(*p).controller == seat).collect();
            let Some(chosen) = choose_cards(game, d, seat, &possible, &possible, 1, 1, "sacrifice") else {
                return false;
            };
            for o in chosen {
                sacrifice(game, o);
            }
        }
        CostDef::DiscardCardCost { filter } => {
            let hand = game.player(seat).hand.clone();
            let possible: Vec<ObjId> =
                hand.iter().copied().filter(|c| game.filter_matches(filter, seat, Some(id), *c)).collect();
            let Some(chosen) = choose_cards(game, d, seat, &hand, &possible, 1, 1, "discard") else {
                return false;
            };
            for o in chosen {
                game.move_to(o, Zone::Graveyard);
            }
        }
        CostDef::PayLifeCost { amount } => lose_life(game, seat, *amount),
        CostDef::DiscardSourceCost => game.move_to(id, Zone::Graveyard),
        CostDef::ManaCost { cost } => {
            let Ok(m) = ManaCost::parse(cost) else { return false };
            return pay_mana(game, d, seat, &m);
        }
        // `OrCost.pay`: one payable cost is paid; with two, ASK (yes: the first)
        CostDef::OrCost { costs } => {
            let usable: Vec<&CostDef> = costs.iter().filter(|c| or_cost_payable(game, seat, id, c)).collect();
            let chosen = match usable.len() {
                0 => return false,
                1 => usable[0],
                _ => match ask(game, d, seat, &Query::YesNo { message: "pay which cost?".into() }) {
                    Answer::Bool(true) => usable[0],
                    Answer::Stop => return false,
                    _ => usable[1],
                },
            };
            return pay_cost(game, d, seat, id, chosen);
        }
    }
    true
}

/// `PlayerImpl.discard`: with no more cards than asked for, all of them; else the player chooses, and they're
/// discarded in the order chosen.
fn discard(game: &mut Game, d: &mut dyn Decider, seat: Seat, amount: i32) -> Vec<ObjId> {
    let hand = game.player(seat).hand.clone();
    let amount = amount.max(0) as usize;
    let chosen = if hand.len() <= amount {
        hand
    } else {
        match choose_cards(game, d, seat, &hand, &hand, amount, amount, "discard") {
            Some(c) => c,
            None => return Vec::new(),
        }
    };
    for c in &chosen {
        game.move_to(*c, Zone::Graveyard);
    }
    chosen
}

/// `PlayerImpl.searchLibrary` with `TargetCardInLibrary.choose`: the whole library is shown and the player picks
/// cards of the filter (PICK_TARGET each), never automatically. Fewer than the least is fine when fewer match; only a
/// search for any card at all can't be left empty. None: stopped.
fn search_library(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    filter: &crate::card::Filter,
    min: usize,
    max: usize,
    source: ObjId,
) -> Option<Vec<ObjId>> {
    let library = game.player(seat).library.clone();
    let possible: Vec<ObjId> =
        library.iter().copied().filter(|c| game.filter_matches(filter, seat, Some(source), *c)).collect();
    let min = min.min(possible.len());
    let required = filter.0.is_empty();
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let before = chosen.len();
        // `HumanPlayer.chooseTarget(cards)`: until complete, or done
        let mut complete = false;
        loop {
            let left: Vec<ObjId> = possible.iter().copied().filter(|c| !chosen.contains(c)).collect();
            let must = required && chosen.len() < min && !left.is_empty();
            match ask(game, d, seat, &Query::Cards { objs: left.clone(), required: must }) {
                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                Answer::Obj(o) if left.contains(&o) => {
                    chosen.push(o);
                    if chosen.len() >= max || left.len() == 1 {
                        complete = true;
                        break;
                    }
                }
                Answer::Stop => return None,
                Answer::Bool(_) if chosen.len() >= min || !must => break,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        }
        if complete || chosen.is_empty() || chosen.len() < min || chosen.len() == before {
            return Some(chosen);
        }
    }
}

/// Whether a spell on the stack can't be countered.
fn cant_be_countered(game: &Game, spell: ObjId) -> bool {
    let controller = game.stack.iter().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.controller);
    let by_static = game.battlefield.iter().any(|src| {
        let s = game.obj(*src);
        Some(s.controller) == controller
            && s.def.abilities.iter().any(|a| match a {
                AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| match e {
                    EffectDef::CantBeCounteredControlledEffect { filter, .. } => {
                        game.filter_matches(filter, s.controller, Some(*src), spell)
                    }
                    _ => false,
                }),
                _ => false,
            })
    });
    by_static || game.obj(spell).def.abilities.iter().any(|a| match a {
        AbilityDef::CantBeCounteredSourceAbility => true,
        AbilityDef::SimpleStaticAbility { effects, .. } => {
            effects.iter().any(|e| matches!(e, EffectDef::CantBeCounteredSourceEffect))
        }
        _ => false,
    })
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
        TargetDef::TargetAnyTarget { .. } => (
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
        TargetDef::TargetPermanent { min, max, .. }
        | TargetDef::TargetCardInGraveyard { min, max, .. }
        | TargetDef::TargetAnyTarget { min, max } => (*min, *max),
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
                if let Some((_, abilities)) = mana_sources_with_kinds(game, seat).into_iter().find(|(s, _)| *s == id) {
                    let choices = auto_pay_abilities(&abilities, &unpaid);
                    let chosen = if choices.len() == 1 {
                        Some(choices[0])
                    } else {
                        // CHOOSE_ABILITY: an ability of that permanent, or nothing
                        let q =
                            Query::ChooseAbility { source: id, abilities: choices.iter().map(|(i, _, _)| *i).collect() };
                        match ask(game, d, seat, &q) {
                            Answer::Ability(o, i) if o == id => choices.iter().find(|(j, _, _)| *j == i).copied(),
                            Answer::Stop => return false,
                            _ => None,
                        }
                    };
                    if let Some((i, m, k)) = chosen {
                        if !activate_mana(game, d, seat, id, i, m, k, &unpaid) {
                            return false;
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
/// Activates a mana ability: taps (and sacrifices) the source, adds the mana, and does the ability's other effects at
/// once. False: stopped while choosing a color.
#[allow(clippy::too_many_arguments)]
fn activate_mana(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    id: ObjId,
    i: usize,
    mut m: Mana,
    k: ManaKind,
    unpaid: &ManaCost,
) -> bool {
    game.obj_mut(id).tapped = true;
    // "tap an untapped permanent you control": asked, unless only enough are left
    for c in game.obj(id).def.abilities[i].mana_costs().to_vec() {
        if let CostDef::TapTargetCost { filter, amount } = c {
            let possible = mana_tap_targets(game, seat, id, &filter);
            let Some(chosen) = choose_cards(game, d, seat, &possible, &possible, amount, amount, "tap") else {
                return false;
            };
            if chosen.len() < amount {
                // the cost wasn't paid: the ability isn't activated
                game.obj_mut(id).tapped = false;
                return true;
            }
            for o in chosen {
                game.obj_mut(o).tapped = true;
            }
        }
    }
    if k == ManaKind::AnyColor {
        let Some(c) = any_color(game, d, seat, unpaid) else { return false };
        m = Mana::of(c, 1);
    }
    for c in game.obj(id).def.abilities[i].mana_costs().to_vec() {
        match c {
            CostDef::SacrificeSourceCost => sacrifice(game, id),
            CostDef::PayLifeCost { amount } => lose_life(game, seat, amount),
            _ => {}
        }
    }
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
    true
}

/// `PermanentImpl.getLethalDamage`: what's left of a creature's toughness (1 from a deathtouch source), a
/// planeswalker's loyalty
fn fight_lethal(game: &Game, id: ObjId, from: ObjId) -> i32 {
    let mut lethal = i32::MAX;
    if game.has_type(id, CardType::Creature) {
        lethal = (game.power_toughness(id).1 - game.obj(id).damage).max(0);
        if game.has_ability(from, &AbilityDef::DeathtouchAbility) {
            lethal = lethal.min(1);
        }
    }
    if game.has_type(id, CardType::Planeswalker) {
        lethal = lethal.min(game.obj(id).counter("loyalty"));
    }
    lethal
}

/// The other untapped permanents of the filter a mana ability's tap cost can tap, in battlefield order.
fn mana_tap_targets(game: &Game, seat: Seat, source: ObjId, filter: &crate::card::Filter) -> Vec<ObjId> {
    matching(game, filter, seat, source)
        .into_iter()
        .filter(|o| *o != source && game.obj(*o).controller == seat && !game.obj(*o).tapped)
        .collect()
}

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
                sacrifice(game, *o);
            }
        }
        // `WarpExileEffect`: exiled, and its owner may cast it from exile on a later turn
        for (o, zcc) in &delayed.warp_exile {
            if game.obj(*o).zone == Zone::Battlefield && game.obj(*o).zcc == *zcc {
                game.move_to(*o, Zone::Exile);
                let turn = game.turn;
                game.obj_mut(*o).warp_from = Some(turn + 1);
            }
        }
        return;
    }
    if let Some((index, zcc)) = item.ability {
        if let Some(a) = def.abilities.get(index) {
            if !game.condition_holds(a.intervening_if(), item.controller, id) || !game.pelt_holds(&item) {
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
        // `Spell.resolve`: an Aura whose target is gone is countered (608.2b); else it enters, then its spell
        // ability's AttachEffect attaches it
        if def.has_subtype("Aura") && !def.spell.targets.is_empty() {
            let Some(legal) = legal_targets(game, &item, &def.spell.targets) else {
                game.move_to(id, Zone::Graveyard);
                return;
            };
            enter_battlefield(game, d, id, item.controller, item.kicked);
            if let Some(Target::Obj(to)) = legal.first().and_then(|t| t.first()).copied() {
                if game.obj(id).zone == Zone::Battlefield && game.obj(to).zone == Zone::Battlefield {
                    let zcc = game.obj(to).zcc;
                    game.obj_mut(id).attached_to = Some((to, zcc));
                }
            }
            return;
        }
        enter_battlefield(game, d, id, item.controller, item.kicked);
        return;
    }
    let zcc = game.obj(id).zcc;
    if item.modes.len() > 1 {
        // `Spell.resolve`: each chosen mode with its own targets; a targeted mode whose targets are all illegal is
        // dropped, and with every targeted mode dropped the spell doesn't resolve (608.2b)
        let mut parts = Vec::new();
        let mut at = 0;
        for k in &item.modes {
            let (effects, targets) = def.spell.mode(*k);
            let mut part = item.clone();
            part.targets = item.targets.get(at..at + targets.len()).map(|t| t.to_vec()).unwrap_or_default();
            part.target_zcc = item.target_zcc.get(at..at + targets.len()).map(|t| t.to_vec()).unwrap_or_default();
            at += targets.len();
            parts.push((effects, targets, part));
        }
        let legal: Vec<Option<Vec<Vec<Target>>>> =
            parts.iter().map(|(_, targets, part)| legal_targets(game, part, targets)).collect();
        let targeted = parts.iter().any(|(_, t, _)| !t.is_empty());
        let fizzles = targeted && parts.iter().zip(&legal).all(|((_, t, _), l)| t.is_empty() || l.is_none());
        if !fizzles {
            for ((effects, _, part), l) in parts.iter().zip(legal) {
                if let Some(l) = l {
                    apply_effects(game, d, part, effects, &l, (id, zcc));
                }
            }
        }
    } else {
        let (effects, targets) = def.spell.mode(item.modes.first().copied().unwrap_or(0));
        if let Some(legal) = legal_targets(game, &item, targets) {
            apply_effects(game, d, &item, effects, &legal, (id, zcc));
        }
    }
    if game.obj(id).zone == Zone::Stack {
        // an adventure goes on an adventure: exiled, its creature castable from there
        if game.obj(id).adventurer.as_ref().is_some_and(|f| f.adventure.is_some()) && !game.obj(id).flashback {
            game.move_to(id, Zone::Exile);
            game.obj_mut(id).on_adventure = true;
        } else {
            game.move_to(id, Zone::Graveyard);
        }
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
    // each effect is one action: what it moves all at once sees the battlefield as it began
    let outer = std::mem::take(&mut game.lookback);
    apply_each_effect(game, d, item, effects, legal, source);
    game.lookback = outer;
}

fn apply_each_effect(
    game: &mut Game,
    d: &mut dyn Decider,
    item: &StackItem,
    effects: &[EffectDef],
    legal: &[Vec<Target>],
    source: (ObjId, u32),
) {
    let legal_target = |i: usize| legal.get(i).and_then(|ts| ts.first().copied());
    let target = legal_target(0);
    // an effect's number, worked out now
    macro_rules! v {
        ($a:expr) => {
            game.value($a, item.controller, source.0, item.x)
        };
    }
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
        game.lookback = game.battlefield.to_vec();
        match e {
            EffectDef::DamageTargetEffect { amount } => {
                let amount = &v!(amount);
                for t in legal.first().cloned().unwrap_or_default() {
                    deal_damage(game, source.0, t, *amount, false);
                }
            }
            EffectDef::DrawCardSourceControllerEffect { amount } => {
                let amount = &v!(amount);
                draw(game, controller, *amount as usize);
            }
            EffectDef::GainLifeEffect { amount } => {
                let amount = &v!(amount);
                gain_life(game, controller, *amount);
            }
            EffectDef::DestroyTargetEffect => {
                for o in &objs {
                    destroy(game, *o);
                }
            }
            EffectDef::CounterTargetEffect => {
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone == Zone::Stack && !cant_be_countered(game, o) {
                        Arc::make_mut(&mut game.stack).retain(|i| i.card != o || i.ability.is_some());
                        game.move_to(o, Zone::Graveyard);
                    }
                }
            }
            EffectDef::BoostTargetEffect { power, toughness, duration } => {
                let power = &v!(power); let toughness = &v!(toughness);
                for o in &objs {
                    boost(game, *o, *power, *toughness, *duration);
                }
            }
            EffectDef::BoostSourceEffect { power, toughness, duration } => {
                let power = &v!(power); let toughness = &v!(toughness);
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    boost(game, o, *power, *toughness, *duration);
                }
            }
            EffectDef::AddCountersSourceEffect { counter, amount } => {
                let n = match amount {
                    None => counter.amount,
                    Some(a) => {
                        let k = v!(a);
                        if counter.amount == 1 { k } else { counter.amount + k.max(0) }
                    }
                };
                let (o, zcc) = source;
                if n > 0 && game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    game.obj_mut(o).add_counters(&counter.name, n);
                }
            }
            EffectDef::AddCountersTargetEffect { counter } => {
                for o in &objs {
                    game.obj_mut(*o).add_counters(&counter.name, counter.amount);
                }
            }
            EffectDef::CreateTokenEffect {
                token,
                amount,
                tapped_and_attacking,
                attacking,
                sacrifice_at_end_of_combat,
                haste_until_end_of_turn,
            } => {
                let amount = &v!(amount);
                let def = Arc::new(token.card());
                token_image(game, token, source.0);
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
                    if *attacking {
                        let defender = game.opponent(controller);
                        game.obj_mut(id).attacking = Some(defender);
                    }
                    if *haste_until_end_of_turn {
                        grant(game, id, &AbilityDef::HasteAbility);
                    }
                    created.push((id, game.obj(id).zcc));
                }
                if *sacrifice_at_end_of_combat {
                    let delayed =
                        Delayed {
                            rule: "At end of combat, sacrifice that token.".into(),
                            sacrifice: created,
                            warp_exile: Vec::new(),
                        };
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
                let amount = &v!(amount);
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
                    sacrifice(game, o);
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
                let (power, toughness) = (&v!(power), &v!(toughness));
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
                let amount = &v!(amount);
                for s in 0..game.seats() {
                    if s != controller {
                        lose_life(game, s, *amount);
                    }
                }
            }
            EffectDef::LoseLifeSourceControllerEffect { amount } => {
                let amount = &v!(amount);
                lose_life(game, controller, *amount);
            }
            EffectDef::LoseLifeTargetEffect { amount } => {
                let amount = &v!(amount);
                for s in &players {
                    lose_life(game, *s, *amount);
                }
            }
            EffectDef::DrawCardTargetEffect { amount } => {
                let amount = &v!(amount);
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
                let amount = &v!(amount);
                for o in matching(game, filter, controller, source.0) {
                    deal_damage(game, source.0, Target::Obj(o), *amount, false);
                }
            }
            EffectDef::BoostAllEffect { power, toughness, duration: Duration::EndOfTurn, filter, exclude_source } => {
                let power = &v!(power); let toughness = &v!(toughness);
                for o in matching(game, filter, controller, source.0) {
                    if !(*exclude_source && o == source.0) {
                        boost(game, o, *power, *toughness, Duration::EndOfTurn);
                    }
                }
            }
            EffectDef::AddCountersAllEffect { counter, filter, amount } => {
                let n = match amount {
                    Some(a) => v!(a).max(0),
                    None => counter.amount,
                };
                for o in matching(game, filter, controller, source.0) {
                    if n > 0 {
                        game.obj_mut(o).add_counters(&counter.name, n);
                    }
                }
            }
            // `FightTargetsEffect`: only with both targets legal; each is dealt the other's power, the first first
            EffectDef::FightTargetsEffect => {
                if let (Some(Target::Obj(a)), Some(Target::Obj(b))) = (target, legal_target(1)) {
                    let creature = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
                    if creature(game, a) && creature(game, b) {
                        let pb = game.power_toughness(b).0;
                        deal_damage(game, b, Target::Obj(a), pb, false);
                        // `fightWithExcess`: the second creature is dealt only lethal damage (`damageWithExcess`)
                        let pa = game.power_toughness(a).0.min(fight_lethal(game, b, a));
                        deal_damage(game, a, Target::Obj(b), pa, false);
                    }
                }
            }
            EffectDef::DestroyAllNamedPermanentsEffect => {
                if let Some(Target::Obj(t)) = target {
                    if game.obj(t).zone == Zone::Battlefield {
                        let name = game.obj(t).name().to_string();
                        let named: Vec<ObjId> =
                            game.battlefield.iter().copied().filter(|b| game.obj(*b).name() == name).collect();
                        for o in named {
                            destroy(game, o);
                        }
                    }
                }
            }
            EffectDef::DoubleCountersSourceEffect { name } => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    let n = game.obj(o).counter(name);
                    if n > 0 {
                        game.obj_mut(o).add_counters(name, n);
                    }
                }
            }
            EffectDef::FeedTheSwarmEffect => {
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone == Zone::Battlefield {
                        let mv = game.obj(o).def.cost.mana_value() as i32;
                        lose_life(game, controller, mv);
                        destroy(game, o);
                    }
                }
            }
            EffectDef::RedcapMeleeEffect => {
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone == Zone::Battlefield {
                        let red = game.obj(o).def.colors.contains(&Color::R);
                        deal_damage(game, source.0, Target::Obj(o), 4, false);
                        if !red {
                            let sac = EffectDef::SacrificeEffect {
                                filter: crate::card::Filter(vec![Pred::Type { t: CardType::Land }]),
                                amount: Amount::Fixed(1),
                            };
                            apply_effects(game, d, item, &[sac], &[vec![Target::Player(controller)]], source);
                        }
                    }
                }
            }
            EffectDef::RamThroughEffect => {
                if let (Some(Target::Obj(a)), Some(Target::Obj(b))) = (target, legal_target(1)) {
                    let on = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield;
                    if on(game, a) && on(game, b) {
                        let power = game.power_toughness(a).0;
                        if power >= 1 {
                            if game.has_ability(a, &AbilityDef::TrampleAbility) {
                                // `damageWithExcess`: what's beyond lethal (1 with deathtouch) goes to the controller
                                let lethal = if game.has_ability(a, &AbilityDef::DeathtouchAbility) {
                                    1
                                } else {
                                    (game.power_toughness(b).1 - game.obj(b).damage).max(0)
                                };
                                let excess = (power - lethal).max(0);
                                deal_damage(game, a, Target::Obj(b), power - excess, false);
                                if excess > 0 {
                                    let p = game.obj(b).controller;
                                    deal_damage(game, a, Target::Player(p), excess, false);
                                }
                            } else {
                                deal_damage(game, a, Target::Obj(b), power, false);
                            }
                        }
                    }
                }
            }
            EffectDef::CullingRitualEffect => {
                let filter = crate::card::Filter(vec![
                    Pred::Not { x: Box::new(Pred::Type { t: CardType::Land }) },
                    Pred::Mv { cmp: Cmp::FewerThan, n: 3 },
                ]);
                let mut n = 0;
                for o in matching(game, &filter, controller, source.0) {
                    if !game.has_ability(o, &AbilityDef::IndestructibleAbility) {
                        destroy(game, o);
                        n += 1;
                    }
                }
                if n > 0 {
                    let q = Query::Amount { min: 0, max: n };
                    let black = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Int(b) if (0..=n).contains(&b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    let pool = &mut game.player_mut(controller).mana_pool;
                    pool.colored[Color::B.index()] += black as u32;
                    pool.colored[Color::G.index()] += (n - black) as u32;
                }
            }
            EffectDef::PlayWithFireEffect => {
                match target {
                    Some(Target::Obj(o)) => {
                        if game.obj(o).zone == Zone::Battlefield {
                            deal_damage(game, source.0, Target::Obj(o), 2, false);
                        }
                    }
                    Some(Target::Player(p)) => {
                        // two damage (nothing prevents it), then scry 1
                        deal_damage(game, source.0, Target::Player(p), 2, false);
                        let scry = EffectDef::ScryEffect { amount: Amount::Fixed(1) };
                        apply_effects(game, d, item, &[scry], legal, source);
                    }
                    None => {}
                }
            }
            EffectDef::ExtinctionEventEffect => {
                let q = Query::YesNo { message: "Odd or even?".to_string() };
                let odd = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                let exiled: Vec<ObjId> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|o| {
                        game.has_type(*o, CardType::Creature)
                            && (game.obj(*o).def.cost.mana_value() % 2 == 1) == odd
                    })
                    .collect();
                for o in exiled {
                    game.move_to(o, Zone::Exile);
                }
            }
            EffectDef::ProphesiedEndEffect => {
                if let Some(o) = objs.first().copied().filter(|o| game.obj(*o).zone == Zone::Battlefield) {
                    let (p, attacking) = (game.obj(o).controller, game.obj(o).attacking.is_some());
                    destroy(game, o);
                    if !attacking {
                        draw(game, p, 1);
                    }
                }
            }
            EffectDef::SoulGuideLanternEffect => {
                for p in (0..game.seats()).filter(|p| *p != controller) {
                    for c in game.player(p).graveyard.clone() {
                        game.move_to(c, Zone::Exile);
                    }
                }
            }
            EffectDef::IngrisStingerquillEffect => {
                if let Some(a) = objs.first().copied() {
                    for p in (0..game.seats()).filter(|p| *p != controller) {
                        deal_damage(game, a, Target::Player(p), 1, false);
                    }
                }
            }
            EffectDef::CantStayAwayEffect => {
                if let Some(c) = objs.first().copied().filter(|c| game.obj(*c).zone == Zone::Graveyard) {
                    enter_battlefield(game, d, c, controller, false);
                    if game.obj(c).zone == Zone::Battlefield {
                        game.obj_mut(c).exile_instead_of_dying = true;
                    }
                }
            }
            EffectDef::ExileTopXMayPlayUntilEffect { amount, choose_one, duration } => {
                let n = v!(amount).max(0) as usize;
                let cards: Vec<ObjId> = game.player(controller).library.iter().take(n).copied().collect();
                for c in &cards {
                    game.move_to(*c, Zone::Exile);
                }
                let mut cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
                if *choose_one && cards.len() > 1 {
                    let Some(chosen) = choose_cards(game, d, controller, &cards, &cards, 1, 1, "choose") else {
                        return;
                    };
                    cards = chosen;
                }
                // through the end of this turn, or of the controller's next turn
                let until = if duration == "EndOfTurn" {
                    game.turn
                } else if game.active == Some(controller) {
                    game.turn + game.seats() as u32
                } else {
                    game.turn + 1
                };
                for c in cards {
                    game.obj_mut(c).may_play = Some((controller, until));
                }
            }
            EffectDef::MagebaneLizardEffect => {
                for p in players.clone() {
                    let n = game.player(p).noncreature_cast as i32;
                    if n > 0 {
                        deal_damage(game, source.0, Target::Player(p), n, false);
                    }
                }
            }
            EffectDef::ConniveSourceEffect => {
                let (o, zcc) = source;
                let p = game.obj(o).controller;
                let there = game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc;
                draw(game, p, 1);
                let n = discard(game, d, p, 1).iter().filter(|c| !game.obj(**c).is(CardType::Land)).count();
                if there && n > 0 {
                    game.obj_mut(o).add_counters("+1/+1", n as i32);
                }
            }
            EffectDef::BoomerangBasicsEffect => {
                if let Some(o) = objs.first().copied().filter(|o| game.obj(*o).zone == Zone::Battlefield) {
                    let yours = game.obj(o).controller == controller;
                    game.move_to(o, Zone::Hand);
                    if yours {
                        draw(game, controller, 1);
                    }
                }
            }
            EffectDef::SunspineLynxEffect => {
                let order: Vec<Seat> = (0..game.seats()).map(|k| (controller + k) % game.seats()).collect();
                let counts: Vec<(Seat, i32)> = order
                    .iter()
                    .map(|p| {
                        let n = game
                            .battlefield
                            .iter()
                            .filter(|o| {
                                game.obj(**o).controller == *p
                                    && game.has_type(**o, CardType::Land)
                                    && !game.obj(**o).def.supertypes.iter().any(|t| t == "BASIC")
                            })
                            .count();
                        (*p, n as i32)
                    })
                    .collect();
                for (p, n) in counts {
                    if n > 0 && !game.player(p).lost {
                        deal_damage(game, source.0, Target::Player(p), n, false);
                    }
                }
            }
            EffectDef::FumigateEffect => {
                let mut n = 0;
                let creatures: Vec<ObjId> =
                    game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
                for o in creatures {
                    if !game.has_ability(o, &AbilityDef::IndestructibleAbility) {
                        destroy(game, o);
                        n += 1;
                    }
                }
                if n > 0 {
                    gain_life(game, controller, n);
                }
            }
            EffectDef::StrategicBetrayalEffect => {
                if let Some(Target::Player(p)) = target {
                    let mut cards = game.player(p).graveyard.clone();
                    let creatures: Vec<ObjId> = game
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
                        .collect();
                    if !creatures.is_empty() {
                        match choose_cards(game, d, p, &creatures, &creatures, 1, 1, "exile") {
                            Some(c) => cards.extend(c),
                            None => return,
                        }
                    }
                    for c in cards {
                        game.move_to(c, Zone::Exile);
                    }
                }
            }
            EffectDef::InvokeDespairEffect => {
                if let Some(Target::Player(p)) = target {
                    for t in [CardType::Creature, CardType::Enchantment, CardType::Planeswalker] {
                        let possible: Vec<ObjId> = game
                            .battlefield
                            .iter()
                            .copied()
                            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, t))
                            .collect();
                        let mut sacrificed = false;
                        if !possible.is_empty() {
                            match choose_cards(game, d, p, &possible, &possible, 1, 1, "sacrifice") {
                                Some(c) => {
                                    for o in c {
                                        if game.obj(o).zone == Zone::Battlefield {
                                            sacrifice(game, o);
                                            sacrificed = true;
                                        }
                                    }
                                }
                                None => return,
                            }
                        }
                        if !sacrificed {
                            lose_life(game, p, 2);
                            draw(game, controller, 1);
                        }
                    }
                }
            }
            EffectDef::ChartACourseEffect => {
                if !game.player(controller).attacked {
                    discard(game, d, controller, 1);
                }
            }
            EffectDef::TemporaryLockdownEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    let filter = crate::card::Filter(vec![
                        Pred::Not { x: Box::new(Pred::Type { t: CardType::Land }) },
                        Pred::Mv { cmp: Cmp::FewerThan, n: 3 },
                    ]);
                    for o in matching(game, &filter, controller, src) {
                        game.move_to(o, Zone::Exile);
                        game.exiled_until_leaves.push((src, zcc, o));
                    }
                }
            }
            EffectDef::PutOnLibrarySourceEffect { top } => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Graveyard && game.obj(o).zcc == zcc {
                    to_library(game, o, *top);
                }
            }
            EffectDef::ReturnSourceFromGraveyardToBattlefieldEffect => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Graveyard && game.obj(o).zcc == zcc {
                    enter_battlefield(game, d, o, controller, false);
                }
            }
            EffectDef::DamageCantBePreventedEffect { .. }
            | EffectDef::CantCastMoreThanOneSpellEffect { .. }
            | EffectDef::PermanentsEnterBattlefieldTappedEffect { .. }
            | EffectDef::CantGainLifeAllEffect
            | EffectDef::ClarionConquerorEffect
            | EffectDef::CastAsThoughItHadFlashAllEffect { .. }
            | EffectDef::NarsetParterOfVeilsEffect
            | EffectDef::GevScaledScorchEntersEffect
            | EffectDef::LovestruckBeastEffect
            | EffectDef::GainPlusOneLifeReplacementEffect
            | EffectDef::SpellCostReductionForEachSourceEffect { .. }
            | EffectDef::CantBeCounteredControlledEffect { .. }
            | EffectDef::DontCauseTriggerEffect
            | EffectDef::SetBasePowerToughnessPlusOneSourceEffect { .. }
            | EffectDef::SpellsCostIncreasingAllEffect { .. }
            | EffectDef::SpellCostReductionSourceEffect { .. } => {}
            EffectDef::LoseLifeOpponentsYouGainLifeLostEffect { amount } => {
                let n = v!(amount);
                let mut lost = 0;
                for p in 0..game.seats() {
                    if p != controller && !game.player(p).lost {
                        lose_life(game, p, n);
                        lost += n.max(0);
                    }
                }
                if lost > 0 {
                    gain_life(game, controller, lost);
                }
            }
            EffectDef::DestroyTargetIfManaValueEffect { n, revolt_n } => {
                if let Some(Target::Obj(o)) = target {
                    let mv = game.obj(o).def.cost.mana_value() as i32;
                    let revolt = game.condition_holds(Some(&Condition::Revolt), controller, source.0);
                    if mv <= *n || (revolt && mv <= *revolt_n) {
                        destroy(game, o);
                    }
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
                    untap(game.obj_mut(*o));
                }
            }
            EffectDef::TapSourceEffect => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    game.obj_mut(o).tapped = true;
                }
            }
            EffectDef::DamageControllerEffect { amount } => {
                let amount = &v!(amount);
                deal_damage(game, source.0, Target::Player(controller), *amount, false);
            }
            EffectDef::ScryEffect { amount } => {
                let amount = &v!(amount);
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
                let amount = &v!(amount);
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
            EffectDef::DrawDiscardControllerEffect { draw: n, discard: k, optional } => {
                if *optional {
                    let q = Query::YesNo { message: "Draw, then discard?".into() };
                    match ask(game, d, controller, &q) {
                        Answer::Bool(true) => {}
                        _ => continue,
                    }
                }
                draw(game, controller, (*n).max(0) as usize);
                discard(game, d, controller, *k);
            }
            EffectDef::DiscardControllerEffect { amount } => {
                let amount = &v!(amount);
                discard(game, d, controller, *amount);
            }
            EffectDef::SacrificeEffect { filter, amount } | EffectDef::SacrificeOpponentsEffect { filter, amount } => {
                let amount = v!(amount);
                let players: Vec<Seat> = if matches!(e, EffectDef::SacrificeEffect { .. }) {
                    match target {
                        Some(Target::Player(p)) => vec![p],
                        _ => vec![],
                    }
                } else {
                    (0..game.seats()).map(|i| (controller + i) % game.seats()).filter(|p| *p != controller).collect()
                };
                let mut chosen_all = Vec::new();
                for p in players {
                    let possible: Vec<ObjId> = matching(game, filter, p, source.0)
                        .into_iter()
                        .filter(|o| game.obj(*o).controller == p)
                        .collect();
                    let n = (amount.max(0) as usize).min(possible.len());
                    if n == 0 {
                        continue;
                    }
                    match choose_cards(game, d, p, &possible, &possible, n, n, "sacrifice") {
                        Some(c) => chosen_all.extend(c),
                        None => return,
                    }
                }
                for o in chosen_all {
                    if game.obj(o).zone == Zone::Battlefield {
                        sacrifice(game, o);
                    }
                }
            }
            // exiled until the source leaves; nothing if it already has
            EffectDef::GetEmblemEffect { emblem } => {
                token_image(game, emblem, source.0);
                let id = game.create_obj(Arc::new(emblem.card()), controller, Zone::Library);
                game.obj_mut(id).trigger_order = (1 << 32) + id.0 as u64;
                game.move_to(id, Zone::Command);
            }
            EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped, counters } => {
                for o in &objs {
                    if game.obj(*o).zone == Zone::Graveyard {
                        enter_battlefield(game, d, *o, controller, false);
                        if let Some(c) = counters {
                            game.obj_mut(*o).add_counters(&c.name, c.amount);
                        }
                        if *tapped {
                            game.obj_mut(*o).tapped = true;
                        }
                    }
                }
            }
            EffectDef::DiscardTargetEffect { amount } => {
                let amount = v!(amount);
                if let Some(Target::Player(p)) = target {
                    discard(game, d, p, amount);
                }
            }
            EffectDef::BecomesCreatureSourceEffect { token, retain, duration } => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    let old = game.obj(src).def.clone();
                    let mut def = (*old).clone();
                    if !*retain {
                        def.types.clear();
                        def.subtypes.clear();
                    }
                    for t in &token.types {
                        if !def.types.contains(t) {
                            def.types.push(*t);
                        }
                    }
                    def.subtypes.extend(token.subtypes.iter().cloned());
                    if !token.colors.is_empty() {
                        def.colors = token.colors.clone();
                    }
                    def.abilities.extend(token.abilities.iter().cloned());
                    def.power = token.power;
                    def.toughness = token.toughness;
                    def.all_creature_types |= token.all_creature_types;
                    let o = game.obj_mut(src);
                    if *duration == Duration::EndOfTurn && o.until_end_of_turn_def.is_none() {
                        o.until_end_of_turn_def = Some(old);
                    }
                    o.def = Arc::new(def);
                }
            }
            EffectDef::ExploreSourceEffect { amount } => {
                let amount = v!(amount);
                for _ in 0..amount.max(0) {
                    if !explore(game, d, controller, source.0) {
                        return;
                    }
                }
            }
            EffectDef::MillCardsTargetEffect { amount } => {
                let n = v!(amount).max(0) as usize;
                if let Some(Target::Player(p)) = target {
                    let top: Vec<ObjId> = game.player(p).library.iter().take(n).copied().collect();
                    for c in top {
                        game.move_to(c, Zone::Graveyard);
                    }
                }
            }
            EffectDef::DiscardEachPlayerEffect { amount, who } => {
                let n = v!(amount).max(0) as usize;
                let mut chosen_all = Vec::new();
                for k in 0..game.seats() {
                    let p = (controller + k) % game.seats();
                    if !Game::whose(who, controller, p) {
                        continue;
                    }
                    let hand = game.player(p).hand.clone();
                    let m = n.min(hand.len());
                    if m == 0 {
                        continue;
                    }
                    match choose_cards(game, d, p, &hand, &hand, m, m, "discard") {
                        Some(c) => chosen_all.extend(c),
                        None => return,
                    }
                }
                for c in chosen_all {
                    game.move_to(c, Zone::Graveyard);
                }
            }
            EffectDef::FabledPassageSearchForLandEffect { filter } => {
                let Some(found) = search_library(game, d, controller, filter, 1, 1, source.0) else { return };
                // it shuffles with the land still in the library
                let mut library = game.player(controller).library.clone();
                game.rng.shuffle(&mut library);
                game.player_mut(controller).library = library;
                if let Some(&c) = found.first() {
                    if !put_cards(game, d, controller, &[c], crate::card::PutCards::BattlefieldTapped) {
                        return;
                    }
                    let lands = game
                        .battlefield
                        .iter()
                        .filter(|p| game.obj(**p).controller == controller && game.has_type(**p, CardType::Land))
                        .count();
                    if lands >= 4 && game.obj(c).zone == Zone::Battlefield {
                        game.obj_mut(c).tapped = false;
                    }
                }
            }
            EffectDef::FabledPassageUntapLandEffect => {}
            EffectDef::MagmaticHellkiteEffect => {
                if let Some(&o) = objs.first() {
                    let player = game.obj(o).controller;
                    destroy(game, o);
                    let basic = crate::card::Filter(vec![
                        crate::card::Pred::Type { t: CardType::Land },
                        crate::card::Pred::Supertype { s: "BASIC".into() },
                    ]);
                    let Some(found) = search_library(game, d, player, &basic, 1, 1, source.0) else { return };
                    if let Some(&c) = found.first() {
                        if !put_cards(game, d, player, &[c], crate::card::PutCards::BattlefieldTapped) {
                            return;
                        }
                        if game.obj(c).zone == Zone::Battlefield {
                            game.obj_mut(c).add_counters("stun", 1);
                        }
                    }
                    let mut library = game.player(player).library.clone();
                    game.rng.shuffle(&mut library);
                    game.player_mut(player).library = library;
                }
            }
            EffectDef::FloodpitsDrownerEffect => {
                let mut perms: Vec<ObjId> = Vec::new();
                let (s, zcc) = source;
                if game.obj(s).zone == Zone::Battlefield && game.obj(s).zcc == zcc {
                    perms.push(s);
                }
                if let Some(&o) = objs.first() {
                    if game.obj(o).zone == Zone::Battlefield {
                        perms.push(o);
                    }
                }
                let same_owner = perms.len() == 2 && game.obj(perms[0]).owner == game.obj(perms[1]).owner;
                let groups: Vec<Vec<ObjId>> =
                    if same_owner { vec![perms.clone()] } else { perms.iter().map(|p| vec![*p]).collect() };
                for mut cards in groups {
                    let owner = game.obj(cards[0]).owner;
                    // `ZonesHandler.chooseOrder`: the owner picks the order to put them on the bottom
                    let mut order = Vec::new();
                    while cards.len() > 1 {
                        match ask(game, d, owner, &Query::Cards { objs: cards.clone(), required: true }) {
                            Answer::Obj(o) if cards.contains(&o) => {
                                cards.retain(|c| *c != o);
                                order.push(o);
                            }
                            Answer::Stop => return,
                            _ => {}
                        }
                    }
                    order.extend(cards);
                    for c in order {
                        to_library(game, c, false);
                    }
                    let mut library = game.player(owner).library.clone();
                    game.rng.shuffle(&mut library);
                    game.player_mut(owner).library = library;
                }
            }
            EffectDef::DamageTargetAndTargetEffect { first, second } => {
                for (i, n) in [(0, *first), (1, *second)] {
                    for t in legal.get(i).cloned().unwrap_or_default() {
                        deal_damage(game, source.0, t, n, false);
                    }
                }
            }
            EffectDef::AddCardTypeSourceEffect { types, duration } => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    let old = game.obj(src).def.clone();
                    let mut def = (*old).clone();
                    for t in types {
                        if !def.types.contains(t) {
                            def.types.push(*t);
                        }
                    }
                    let o = game.obj_mut(src);
                    if *duration == Duration::EndOfTurn && o.until_end_of_turn_def.is_none() {
                        o.until_end_of_turn_def = Some(old);
                    }
                    o.def = Arc::new(def);
                }
            }
            EffectDef::LoseGainEffect => {
                gain_life(game, controller, 2);
                for s in &players {
                    lose_life(game, *s, 2);
                }
            }
            EffectDef::ScavengingOozeEffect => {
                if let Some(&o) = objs.first() {
                    game.move_to(o, Zone::Exile);
                    if game.obj(o).is(CardType::Creature) {
                        let (s, zcc) = source;
                        if game.obj(s).zone == Zone::Battlefield && game.obj(s).zcc == zcc {
                            game.obj_mut(s).add_counters("+1/+1", 1);
                        }
                        gain_life(game, controller, 1);
                    }
                }
            }
            EffectDef::ExileGraveyardAllPlayersEffect => {
                for p in 0..game.seats() {
                    for c in game.player(p).graveyard.clone() {
                        game.move_to(c, Zone::Exile);
                    }
                }
            }
            EffectDef::ExileGraveyardAllTargetPlayerEffect => {
                if let Some(Target::Player(p)) = target {
                    for c in game.player(p).graveyard.clone() {
                        game.move_to(c, Zone::Exile);
                    }
                }
            }
            EffectDef::CantBeBlockedTargetEffect { .. } => {
                for o in &objs {
                    grant(game, *o, &AbilityDef::CantBeBlockedSourceAbility);
                }
            }
            EffectDef::ExploreTargetEffect => {
                for o in &objs {
                    if !explore(game, d, controller, *o) {
                        return;
                    }
                }
            }
            EffectDef::ExileUntilSourceLeavesEffect => {
                let (src, zcc) = source;
                if let Some(Target::Obj(o)) = target {
                    if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                        game.move_to(o, Zone::Exile);
                        game.exiled_until_leaves.push((src, zcc, o));
                    }
                }
            }
            EffectDef::DiscardCardYouChooseTargetEffect { filter } => {
                if let Some(Target::Player(p)) = target {
                    let hand = game.player(p).hand.clone();
                    let possible: Vec<ObjId> = hand
                        .iter()
                        .copied()
                        .filter(|c| game.filter_matches(filter, controller, Some(source.0), *c))
                        .collect();
                    if !possible.is_empty() {
                        match choose_cards(game, d, controller, &hand, &possible, 1, 1, "choose") {
                            Some(chosen) => {
                                for c in chosen {
                                    game.move_to(c, Zone::Graveyard);
                                }
                            }
                            None => return,
                        }
                    }
                }
            }
            EffectDef::SearchLibraryEffect { filter, min, max, to, optional, target_controller } => {
                let searcher = if *target_controller {
                    match target {
                        Some(Target::Obj(o)) => game.obj(o).controller,
                        Some(Target::Player(p)) => p,
                        None => continue,
                    }
                } else {
                    controller
                };
                if *optional || *target_controller {
                    let q = Query::YesNo { message: "Search your library?".into() };
                    if ask(game, d, searcher, &q) != Answer::Bool(true) {
                        continue;
                    }
                }
                let Some(found) = search_library(game, d, searcher, filter, *min, *max, source.0) else { return };
                if !found.is_empty() && !put_cards(game, d, searcher, &found, *to) {
                    return;
                }
                let mut library = game.player(searcher).library.clone();
                game.rng.shuffle(&mut library);
                game.player_mut(searcher).library = library;
            }
            EffectDef::ReturnCardChosenFromGraveyardEffect { filter, optional } => {
                let graveyard = game.player(controller).graveyard.clone();
                let possible: Vec<ObjId> = graveyard
                    .iter()
                    .copied()
                    .filter(|c| game.filter_matches(filter, controller, Some(source.0), *c))
                    .collect();
                if !possible.is_empty() {
                    let min = if *optional { 0 } else { 1 };
                    match choose_cards(game, d, controller, &graveyard, &possible, min, 1, "choose") {
                        Some(chosen) => {
                            for c in chosen {
                                game.move_to(c, Zone::Hand);
                            }
                        }
                        None => return,
                    }
                }
            }
            EffectDef::RevealLibraryPickControllerEffect { amount, pick, filter, optional } => {
                let amount = &v!(amount);
                let n = (*amount).max(0) as usize;
                let cards: Vec<ObjId> = game.player(controller).library.iter().take(n).copied().collect();
                let possible: Vec<ObjId> = cards
                    .iter()
                    .copied()
                    .filter(|c| game.filter_matches(filter, controller, Some(source.0), *c))
                    .collect();
                let number = (*pick).min(possible.len());
                let mut picked = Vec::new();
                if number > 0 {
                    let yes = !*optional || {
                        let q = Query::YesNo { message: "Put a card into your hand?".into() };
                        loop {
                            match ask(game, d, controller, &q) {
                                Answer::Bool(b) => break b,
                                Answer::Stop => return,
                                _ => {}
                            }
                        }
                    };
                    if yes {
                        match choose_cards(game, d, controller, &cards, &possible, number, number, "choose") {
                            Some(chosen) => picked = chosen,
                            None => return,
                        }
                    }
                }
                for c in &picked {
                    game.move_to(*c, Zone::Hand);
                }
                for c in cards.iter().filter(|c| !picked.contains(c)) {
                    game.move_to(*c, Zone::Graveyard);
                }
            }
            EffectDef::DrawDiscardTargetEffect { draw: n, discard: k } => {
                for p in players.clone() {
                    draw(game, p, *n as usize);
                    discard(game, d, p, *k);
                }
            }
            EffectDef::CreateTokenTargetEffect { token } => {
                let def = Arc::new(token.card());
                token_image(game, token, source.0);
                for p in players.clone() {
                    let id = game.create_obj(def.clone(), p, Zone::Library);
                    let o = game.obj_mut(id);
                    o.token = true;
                    o.trigger_order = (1 << 32) + id.0 as u64;
                    enter_battlefield(game, d, id, p, false);
                }
            }
            EffectDef::BasicManaEffect { colored, colorless } => {
                let pool = &mut game.player_mut(controller).mana_pool;
                for (k, n) in colored.iter().enumerate() {
                    pool.colored[k] += n;
                }
                pool.colorless += colorless;
            }
            EffectDef::CreateTokenControllerTargetEffect { token, amount, tapped } => {
                let def = Arc::new(token.card());
                token_image(game, token, source.0);
                let to = objs.first().map(|o| game.obj(*o).controller);
                for p in to.into_iter().flat_map(|p| std::iter::repeat_n(p, *amount as usize)) {
                    let id = game.create_obj(def.clone(), p, Zone::Library);
                    let o = game.obj_mut(id);
                    o.token = true;
                    o.trigger_order = (1 << 32) + id.0 as u64;
                    enter_battlefield(game, d, id, p, false);
                    if *tapped {
                        game.obj_mut(id).tapped = true;
                    }
                }
            }
            EffectDef::ReturnToHandFromBattlefieldAllEffect { filter } => {
                for o in matching(game, filter, controller, source.0) {
                    game.move_to(o, Zone::Hand);
                }
            }
            EffectDef::DiscardHandControllerEffect => {
                for c in game.player(controller).hand.clone() {
                    game.move_to(c, Zone::Graveyard);
                }
            }
            EffectDef::MillThenPutInHandEffect { amount, filter, optional } => {
                let amount = &v!(amount);
                let n = (*amount).max(0) as usize;
                let milled: Vec<ObjId> = game.player(controller).library.iter().take(n).copied().collect();
                for c in &milled {
                    game.move_to(*c, Zone::Graveyard);
                }
                let possible: Vec<ObjId> = milled
                    .iter()
                    .copied()
                    .filter(|c| game.filter_matches(filter, controller, Some(source.0), *c))
                    .collect();
                if !possible.is_empty() {
                    let min = if *optional { 0 } else { 1 };
                    match choose_cards(game, d, controller, &milled, &possible, min, 1, "choose") {
                        Some(chosen) => {
                            for c in chosen {
                                game.move_to(c, Zone::Hand);
                            }
                        }
                        None => return,
                    }
                }
            }
            EffectDef::MillCardsControllerEffect { amount } => {
                let amount = &v!(amount);
                let n = (*amount).max(0) as usize;
                let top: Vec<ObjId> = game.player(controller).library.iter().take(n).copied().collect();
                for c in top {
                    game.move_to(c, Zone::Graveyard);
                }
            }
            // ward against an ability: counter (remove) it unless its controller pays
            EffectDef::CounterUnlessPaysEffect { cost, .. } if item.ward_of.is_some() => {
                let (card, ability) = item.ward_of.unwrap();
                let Some(at) = game.stack.iter().rposition(|i| i.card == card && i.ability == Some(ability)) else {
                    continue;
                };
                let payer = game.stack[at].controller;
                let Some(paid) = pay_unless(game, d, payer, cost) else { return };
                if !paid {
                    Arc::make_mut(&mut game.stack).remove(at);
                }
            }
            EffectDef::CounterUnlessPaysEffect { cost, exile } => {
                let Some(Target::Obj(spell)) = target else { continue };
                if game.obj(spell).zone != Zone::Stack {
                    continue;
                }
                let payer = game.stack.iter().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.controller);
                let Some(payer) = payer else { continue };
                let Some(paid) = pay_unless(game, d, payer, cost) else { return };
                if !paid && !cant_be_countered(game, spell) {
                    Arc::make_mut(&mut game.stack).retain(|i| i.card != spell || i.ability.is_some());
                    game.move_to(spell, if *exile { Zone::Exile } else { Zone::Graveyard });
                }
            }
            // resolving, "{this} gains … until end of turn"; as a static ability it's `Game::has_ability`'s
            EffectDef::GainAbilitySourceEffect { ability, duration } => {
                if *duration == Duration::EndOfTurn
                    && game.obj(source.0).zone == Zone::Battlefield
                    && game.obj(source.0).zcc == source.1
                {
                    grant(game, source.0, ability);
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
            | EffectDef::ConditionalContinuousEffect { .. }
            | EffectDef::SetBasePowerToughnessSourceEffect { .. }
            | EffectDef::CantBeCounteredSourceEffect
            | EffectDef::AddCardSubTypeSourceEffect
            | EffectDef::BoostAllEffect { .. }
            | EffectDef::TapSourceUnlessPaysEffect { .. }
            | EffectDef::AttachEffect
            | EffectDef::BoostEnchantedEffect { .. }
            | EffectDef::CantAttackBlockAttachedEffect
            | EffectDef::GraveyardFromAnywhereExileReplacementEffect
            | EffectDef::SetBasePowerSourceEffect { .. } => {}
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

/// `ExploreSourceEffect.doOneExplore`: reveal the top card; a land goes to hand, otherwise the explorer gets a +1/+1
/// counter and its controller may put the card into their graveyard (ASK). False: the decider stopped.
fn explore(game: &mut Game, d: &mut dyn Decider, seat: Seat, explorer: ObjId) -> bool {
    let on_battlefield = |g: &Game| g.obj(explorer).zone == Zone::Battlefield;
    let Some(card) = game.player(seat).library.first().copied() else {
        if on_battlefield(game) {
            game.obj_mut(explorer).add_counters("+1/+1", 1);
        }
        return true;
    };
    if game.obj(card).is(CardType::Land) {
        game.move_to(card, Zone::Hand);
        return true;
    }
    if on_battlefield(game) {
        game.obj_mut(explorer).add_counters("+1/+1", 1);
    }
    let message = format!("Put {} in your graveyard?", game.obj(card).name());
    match ask(game, d, seat, &Query::YesNo { message }) {
        Answer::Bool(true) => game.move_to(card, Zone::Graveyard),
        Answer::Stop => return false,
        _ => {}
    }
    true
}

/// Sacrifices a permanent: it goes to its owner's graveyard (dies), then `SACRIFICED_PERMANENT` for "whenever
/// you sacrifice" triggers.
fn sacrifice(game: &mut Game, id: ObjId) {
    if game.obj(id).zone != Zone::Battlefield {
        return;
    }
    let who = game.obj(id).controller;
    let sources = game.trigger_sources();
    let fired = game.triggers(sources, |g, src, you, t| {
        matches!(t, Trigger::Sacrificed { filter, who: w } if Game::whose(w, you, who) && g.filter_matches(filter, you, Some(src), id))
    });
    game.move_to(id, Zone::Graveyard);
    for mut t in fired {
        if set_target_pointer(game, &t) {
            t.targets = vec![vec![Target::Player(who)]];
            t.target_zcc = vec![vec![0]];
        }
        game.pending.push(t);
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
    // a permanent spell resolving was cast (`CastFromEverywhereSourceCondition`)
    let cast = game.obj(id).zone == Zone::Stack;
    game.move_to(id, Zone::Battlefield);
    let def = game.obj(id).def.clone();
    game.obj_mut(id).controller = controller;
    let tapped_by_static = game.any_static(|g, src, e| match e {
        EffectDef::PermanentsEnterBattlefieldTappedEffect { filter } => {
            g.filter_matches(filter, g.obj(src).controller, Some(src), id)
        }
        _ => false,
    });
    let zcc = {
        let o = game.obj_mut(id);
        o.controller = controller;
        if tapped_by_static || def.abilities.iter().any(|a| a.is_enters_tapped()) {
            o.tapped = true;
        }
        if let Some(n) = def.loyalty.filter(|n| *n > 0) {
            o.add_counters("loyalty", n);
        }
        o.kicked = kicked;
        o.cast = cast;
        o.zcc
    };
    // `WarpAbilityWatcher`: a warped permanent is exiled at the beginning of the next end step
    if game.obj(id).warped {
        let delayed = Delayed {
            rule: "At the beginning of the next end step, exile this creature if it was cast for its warp cost.".into(),
            sacrifice: Vec::new(),
            warp_exile: vec![(id, zcc)],
        };
        game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, controller) });
    }
    // Gev, Scaled Scorch: other creatures you control enter with counters
    if game.has_type(id, CardType::Creature) {
        let mut n = 0;
        let lost = (0..game.seats()).filter(|p| *p != controller && game.player(*p).life_lost > 0).count() as i32;
        game.any_static(|g, src, e| {
            if matches!(e, EffectDef::GevScaledScorchEntersEffect) && src != id && g.obj(src).controller == controller {
                n += lost;
            }
            false
        });
        if n > 0 {
            game.obj_mut(id).add_counters("+1/+1", n);
        }
    }
    // an Aura put onto the battlefield without being cast: its controller picks what it enchants (303.4f)
    if !cast && def.has_subtype("Aura") {
        if let Some(t) = def.spell.targets.first() {
            let rule = format!("attach {} to", def.name);
            if let Some(chosen) = choose_target(game, d, controller, t, Some(id), &rule, false) {
                if let Some(Target::Obj(to)) = chosen.first().copied() {
                    let zcc = game.obj(to).zcc;
                    game.obj_mut(id).attached_to = Some((to, zcc));
                }
            }
        }
    }
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
                        lose_life(game, controller, *life);
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
    // Torpor Orb: a creature entering triggers nothing
    let orb = game.battlefield.iter().any(|s| {
        game.obj(*s).def.abilities.iter().any(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => {
                effects.iter().any(|e| matches!(e, EffectDef::DontCauseTriggerEffect))
            }
            _ => false,
        })
    });
    if orb && game.has_type(id, CardType::Creature) {
        sources.clear();
    }
    for p in game.players.iter() {
        sources.extend(p.graveyard.iter().copied());
    }
    let fired = game.triggers(sources, |g, src, you, t| match t {
        Trigger::Enters | Trigger::EntersOrDies | Trigger::EntersOrAttacks => {
            src == id && g.obj(src).zone == Zone::Battlefield
        }
        Trigger::EntersAll { filter, include_self, graveyard } => {
            *graveyard == (g.obj(src).zone == Zone::Graveyard)
                && ((src == id && *include_self) || g.filter_matches(filter, you, Some(src), id))
        }
        Trigger::PeltCollector => src != id && g.obj(src).zone == Zone::Battlefield && controller == you,
        _ => false,
    });
    let mut fired = game.pelt_collector(fired, id);
    for t in fired.iter_mut() {
        let a = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i).cloned());
        if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. })) {
            t.targets = vec![vec![Target::Obj(id)]];
            t.target_zcc = vec![vec![zcc]];
        }
        if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. })) {
            t.targets = vec![vec![Target::Player(controller)]];
            t.target_zcc = vec![vec![0]];
        }
    }
    game.pending.extend(fired);
}

/// The creatures that can crew `vehicle`: its controller's other untapped creatures
fn crew_candidates(game: &Game, seat: Seat, vehicle: ObjId) -> Vec<ObjId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|c| {
            *c != vehicle
                && game.obj(*c).controller == seat
                && !game.obj(*c).tapped
                && game.has_type(*c, CardType::Creature)
        })
        .collect()
}

/// Untaps a permanent, unless it has a stun counter: then one comes off instead
fn untap(o: &mut crate::state::Obj) {
    if o.tapped && o.counter("stun") > 0 {
        o.add_counters("stun", -1);
    } else {
        o.tapped = false;
    }
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
        if !game.returning.is_empty() {
            let mut back = std::mem::take(&mut game.returning);
            // `AuraReplacementEffect` puts an Aura onto the battlefield as its move is replaced, before the rest
            back.sort_by_key(|c| !game.obj(*c).def.has_subtype("Aura"));
            for c in back {
                if game.obj(c).zone == Zone::Exile {
                    let owner = game.obj(c).owner;
                    enter_battlefield(game, d, c, owner, false);
                }
            }
            continue;
        }
        if d.stopped() || game.pending.is_empty() {
            return;
        }
        let mut pending = std::mem::take(&mut game.pending);
        // an intervening "if" is checked as it triggers (and again as it resolves)
        pending.retain(|t| {
            let ability = t.ability.and_then(|(i, _)| game.obj(t.card).def.abilities.get(i));
            game.condition_holds(ability.and_then(|a| a.intervening_if()), t.controller, t.card) && game.pelt_holds(t)
        });
        // a trigger limit each turn
        pending.retain(|t| {
            let Some((i, _)) = t.ability else { return true };
            let Some(AbilityDef::TriggeredAbility { limit_per_turn: Some(n), .. }) = game.obj(t.card).def.abilities.get(i)
            else {
                return true;
            };
            let seen = game.triggered_this_turn.iter().filter(|x| **x == (t.card, i)).count() as u32;
            if seen >= *n {
                return false;
            }
            game.triggered_this_turn.push((t.card, i));
            true
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

/// `TokenImpl.generateTokenInfo`: creating tokens (or an emblem) picks an image with the game's random when there are
/// two or more to pick from.
fn token_image(game: &mut Game, token: &crate::card::TokenDef, source: ObjId) {
    let n = token.image_choices(&game.obj(source).def.set);
    if n >= 2 {
        game.rng.next_int_bound(n as i32);
    }
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
            lose_life(game, s, amount);
            if !combat {
                damaged_opponent_triggers(game, s);
            }
        }
        Target::Obj(o) => {
            if game.obj(o).zone != Zone::Battlefield {
                return 0;
            }
            let deathtouch = src.has(&AbilityDef::DeathtouchAbility);
            let creature = game.has_type(o, CardType::Creature);
            let planeswalker = game.has_type(o, CardType::Planeswalker);
            let t = game.obj_mut(o);
            if creature {
                t.damage += amount;
                if deathtouch {
                    t.deathtouched = true;
                }
            }
            // damage to a planeswalker removes that many loyalty counters
            if planeswalker {
                let n = amount.min(t.counter("loyalty"));
                t.add_counters("loyalty", -n);
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
    if let Target::Player(p) = to {
        let fired = game.triggers([source], |_, _, you, t| {
            matches!(t, Trigger::DealsDamageToOpponent { only_combat } if p != you && (combat || !*only_combat))
        });
        for mut t in fired {
            if set_target_pointer(game, &t) {
                t.targets = vec![vec![Target::Player(p)]];
                t.target_zcc = vec![vec![0]];
            }
            game.pending.push(t);
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
    let sources: Vec<ObjId> = game.trigger_sources();
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
/// Life lost (damage, paying life, "loses life"), counted for the turn (`PlayerLostLifeWatcher`).
pub fn lose_life(game: &mut Game, seat: Seat, amount: i32) {
    let p = game.player_mut(seat);
    p.life -= amount;
    if amount > 0 {
        p.life_lost += amount;
    }
}

pub fn gain_life(game: &mut Game, seat: Seat, amount: i32) {
    if amount <= 0 || game.any_static(|_, _, e| matches!(e, EffectDef::CantGainLifeAllEffect)) {
        return;
    }
    let plus = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == seat)
        .flat_map(|b| game.obj(*b).def.abilities.iter())
        .filter(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => {
                effects.iter().any(|e| matches!(e, EffectDef::GainPlusOneLifeReplacementEffect))
            }
            _ => false,
        })
        .count() as i32;
    let amount = amount + plus;
    game.player_mut(seat).life += amount;
    game.player_mut(seat).life_gains += 1;
    let first = game.player(seat).life_gains == 1;
    let sources: Vec<ObjId> = game.trigger_sources();
    let fired = game.triggers(sources, |_, _, you, t| {
        you == seat && (matches!(t, Trigger::YouGainLife) || (first && matches!(t, Trigger::YouGainLifeFirstTime)))
    });
    // the life gained is the trigger's "that much" (`SavedGainedLifeValue`), read as its X
    game.pending.extend(fired.into_iter().map(|t| StackItem { x: amount, ..t }));
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
        && (!o.sick || game.has_ability(id, &AbilityDef::HasteAbility))
        && !o.has(&AbilityDef::DefenderAbility)
        && !game.cant_attack_or_block(id)
        && !game.cant_attack_alone(id)
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
                let Some(pw) = choose_defender(game, d, seat, defender) else { return };
                for id in creatures {
                    attack(game, id, defender, pw);
                }
            }
            Answer::Obj(id) => {
                if creatures.contains(&id) {
                    let Some(pw) = choose_defender(game, d, seat, defender) else { return };
                    attack(game, id, defender, pw);
                } else if game.obj(id).attacking.is_some() && game.stack.is_empty() {
                    let vigilance = game.obj(id).has(&AbilityDef::VigilanceAbility);
                    let o = game.obj_mut(id);
                    o.attacking = None;
                    o.attacking_pw = None;
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

/// `HumanPlayer.selectDefender`: with the defending player the only defender there's no question; with their
/// planeswalkers too, PICK_TARGET (required: anything else asks again). Some(None): the player; None: stopped.
fn choose_defender(game: &mut Game, d: &mut dyn Decider, seat: Seat, defender: Seat) -> Option<Option<(ObjId, u32)>> {
    let pws: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| game.obj(*p).controller == defender && game.has_type(*p, CardType::Planeswalker))
        .collect();
    if pws.is_empty() {
        return Some(None);
    }
    loop {
        match ask(game, d, seat, &Query::Target { players: vec![defender], objs: pws.clone(), required: true }) {
            Answer::Player(p) if p == defender => return Some(None),
            Answer::Obj(o) if pws.contains(&o) => return Some(Some((o, game.obj(o).zcc))),
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}

fn attack(game: &mut Game, id: ObjId, defender: Seat, pw: Option<(ObjId, u32)>) {
    let vigilance = game.obj(id).has(&AbilityDef::VigilanceAbility);
    let o = game.obj_mut(id);
    o.attacking = Some(defender);
    o.attacking_pw = pw;
    if !vigilance {
        o.tapped = true;
    }
}

fn can_block(game: &Game, blocker: ObjId, attacker: ObjId) -> bool {
    if game.has_ability(attacker, &AbilityDef::CantBeBlockedSourceAbility)
        || game.cant_attack_or_block(blocker)
        || game.has_ability(blocker, &AbilityDef::CantBlockAbility)
        || (game.has_ability(attacker, &AbilityDef::DauntAbility) && game.power_toughness(blocker).0 <= 2)
    {
        return false;
    }
    if game.has_ability(blocker, &AbilityDef::CanBlockOnlyFlyingAbility)
        && !game.has_ability(attacker, &AbilityDef::FlyingAbility)
    {
        return false;
    }
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
        // what an unblocked (or trampling) attacker damages: its player or planeswalker; nothing if that's gone
        let attacked = match game.obj(a).attacking_pw {
            None => Some(Target::Player(defender)),
            Some((pw, zcc)) => {
                (game.obj(pw).zone == Zone::Battlefield && game.obj(pw).zcc == zcc).then_some(Target::Obj(pw))
            }
        };
        if !game.obj(a).blocked {
            if let Some(to) = attacked {
                hits.push((a, to, damage));
            }
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
                if let Some(to) = attacked {
                    hits.push((a, to, over));
                }
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
        if game.obj(id).loyalty_used {
            game.obj_mut(id).loyalty_used = false;
        }
        if game.obj(id).until_end_of_turn_def.is_some() {
            let o = game.obj_mut(id);
            o.def = o.until_end_of_turn_def.take().unwrap();
        }
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
            // nobody is marked as the winner yet; the fingerprint hides the loser's cards
            game.over = true;
            return;
        }
        let dying: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                let o = game.obj(*id);
                if game.has_type(*id, CardType::Planeswalker) && o.counter("loyalty") <= 0 {
                    return true;
                }
                // 704.5m: an Aura attached to nothing, or to what it can't enchant
                if o.def.has_subtype("Aura") && !o.is(CardType::Creature) {
                    let Some(to) = game.attached(*id) else { return true };
                    if let Some(TargetDef::TargetPermanent { filter, .. }) = o.def.spell.targets.first() {
                        if !game.filter_matches(filter, o.controller, Some(*id), to) {
                            return true;
                        }
                    }
                }
                if !o.is(CardType::Creature) {
                    return false;
                }
                let (_, t) = game.power_toughness(*id);
                t <= 0
                    || (!game.has_ability(*id, &AbilityDef::IndestructibleAbility)
                        && (o.damage >= t || (o.deathtouched && o.damage > 0)))
            })
            .collect();
        game.lookback = game.battlefield.to_vec();
        for id in dying {
            game.move_to(id, Zone::Graveyard);
            any = true;
        }
        game.lookback.clear();
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
