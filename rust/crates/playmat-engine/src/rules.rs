//! The rules: turns and steps, priority, playing lands and casting spells, paying mana, resolving, combat,
//! state-based actions. The order of everything a player sees follows Java's `GameImpl.playPriority` and
//! `HumanPlayer` with the client's default settings, so a shadow game replays answer for answer.

use crate::card::{
    AbilityDef, Amount, CardDef, CardType, Cmp, Color, Condition, CostDef, Duration, EffectDef, Mana, ManaCost,
    ManaKind, Pred, TargetDef, Trigger,
};
use crate::decide::{Answer, Decider, Query};
use crate::log;
use crate::state::{Boost, Delayed, EventKind, Game, GameEvent, Grant, ObjId, Seat, StackItem, Step, Target, Zone};
use std::sync::Arc;

const MAX_HAND: usize = 7;

/// Plays from the first priority of the game (as `ShadowGame::start_game` leaves it) until a player loses, the decider
/// stops, or turn `stop_on_turn` would begin (Java's `GameOptions.stopOnTurn`; 0 plays on).
pub fn play_from_priority(game: &mut Game, d: &mut dyn Decider, stop_on_turn: u32) {
    play_on(game, d, stop_on_turn, false)
}

/// Plays on from a copy of a game taken while a seat was asked `Query::Priority` (the AI's search): that seat is asked
/// again, with the passes made so far kept.
pub fn play_from_priority_question(game: &mut Game, d: &mut dyn Decider, stop_on_turn: u32) {
    play_on(game, d, stop_on_turn, true)
}

fn play_on(game: &mut Game, d: &mut dyn Decider, stop_on_turn: u32, mut resume: bool) {
    let mut first = true;
    while !game.over && !d.stopped() {
        let step = match game.step {
            Some(s) => s,
            None => return,
        };
        if !first {
            game.step_start = true;
            begin_step(game, d, step);
        }
        game.step_start = false;
        first = false;
        if game.over || d.stopped() {
            break;
        }
        // untap has no priority; cleanup only when 514.3a applies
        if step == Step::Cleanup {
            cleanup_priority(game, d, std::mem::take(&mut resume));
        } else if step != Step::Untap {
            priority_round(game, d, std::mem::take(&mut resume));
        }
        if game.over || d.stopped() {
            break;
        }
        end_step(game, step);
        if d.stop_after(game, step) {
            return;
        }
        if step == Step::Upkeep && game.turn == 1 && game.log.is_some() {
            // the starting player's draw (Java's turn mod for it)
            let line = format!("{} skips Draw step", log::player(game.active.unwrap_or(0)));
            log::inform(game, line);
        }
        match next_step(game, step) {
            Some(s) => game.step = Some(s),
            None => {
                if stop_on_turn != 0 && game.turn + 1 >= stop_on_turn {
                    return;
                }
                game.turn += 1;
                // a controlled turn over: that player takes an extra turn after it (Emrakul's)
                let ended = game.active.unwrap_or(0);
                if let Some(i) = game.controlled_by.iter().position(|(p, _)| *p == ended) {
                    game.controlled_by.remove(i);
                    game.extra_turns.push(ended);
                }
                game.active = Some(match game.extra_turns.pop() {
                    Some(s) => s,
                    None => game.opponent(game.active.unwrap_or(0)),
                });
                let now = game.active.unwrap_or(0);
                if let Some(i) = game.control_next_turn.iter().position(|(p, _)| *p == now) {
                    let c = game.control_next_turn.remove(i);
                    game.controlled_by.push(c);
                }
                game.step = Some(Step::Untap);
            }
        }
    }
    if game.over {
        traced(game, d);
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

/// 514.3a (`EndPhase.playStep`): after the cleanup actions, if state-based actions were performed or triggers were
/// put on the stack, the active player gets priority, and once all pass with the stack empty another cleanup step
/// begins (Java's extra `CleanupStep` turn mod), which checks again. `resume`: a copy taken while a seat was asked at
/// this priority plays on from it.
fn cleanup_priority(game: &mut Game, d: &mut dyn Decider, mut resume: bool) {
    loop {
        if !std::mem::take(&mut resume) {
            if !check_state_and_triggered(game, d) || game.over || d.stopped() {
                return;
            }
            if log::kept(game) {
                log::inform(
                    game,
                    "State-based actions or triggers happened on cleanup step, so players get priority due 514.3a"
                        .into(),
                );
            }
            priority_round(game, d, false);
        } else {
            priority_round(game, d, true);
        }
        if game.over || d.stopped() {
            return;
        }
        end_step(game, Step::Cleanup);
        if log::kept(game) {
            let line = format!("{} takes an extra Cleanup step", log::player(game.active.unwrap_or(0)));
            log::inform(game, line);
        }
        game.step_start = true;
        begin_step(game, d, Step::Cleanup);
        game.step_start = false;
        if game.over || d.stopped() {
            return;
        }
    }
}

/// The turn-based actions at the start of a step.
/// An event only the Career's tally counts (`EventKind::shown`), while events are kept
fn note(
    game: &mut Game,
    kind: EventKind,
    player: Option<Seat>,
    source: Option<ObjId>,
    target: Option<Target>,
    amount: i32,
) {
    if game.events.is_some() {
        game.record(GameEvent { player, source, target, amount, ..GameEvent::new(kind) });
    }
}

/// `source`'s effect countered `spell`
fn note_countered(game: &mut Game, spell: ObjId, source: ObjId) {
    let player = Some(game.obj(spell).controller);
    note(game, EventKind::Countered, player, Some(source), Some(Target::Obj(spell)), 0);
}

fn begin_step(game: &mut Game, d: &mut dyn Decider, step: Step) {
    let active = game.active.unwrap_or(0);
    match step {
        Step::Untap => {
            note(game, EventKind::Turn, Some(active), None, None, 0);
            // 726.2: the previous turn's active player cast no spells: day becomes night; two or more: night
            // becomes day (`UntapStep.handleDayNight`, before untapping)
            if game.day.is_some() && game.turn > 1 {
                let previous = game.player((active + game.seats() - 1) % game.seats()).spells_cast;
                let change = match game.day {
                    Some(true) => previous == 0,
                    _ => previous >= 2,
                };
                if change {
                    game.day = game.day.map(|d| !d);
                    day_night(game);
                }
            }
            for s in 0..game.seats() {
                let p = game.player_mut(s);
                p.lands_played = 0;
                p.attacked = false;
                p.life_lost = 0;
                p.drawn = 0;
                p.noncreature_cast = 0;
                p.flier_cast = false;
                p.permanent_left = false;
                p.life_gains = 0;
                p.life_gained = 0;
                p.spells_cast = 0;
                p.instants_sorceries_cast = 0;
                p.sacrificed = 0;
                p.expended = 0;
                p.lands_entered = 0;
            }
            // "until your next turn" ends as it begins
            game.attacks_opponent.retain(|(s, _)| *s != active);
            game.flash_until_next_turn.retain(|(s, _, _)| *s != active);
            if game.grants.iter().any(|g| g.until_turn_of == Some(active)) {
                Arc::make_mut(&mut game.grants).retain(|g| g.until_turn_of != Some(active));
            }
            game.triggered_this_turn.clear();
            game.creature_died = false;
            game.activated_this_turn.clear();
            for id in Arc::clone(&game.battlefield).iter().copied() {
                if game.obj(id).controller == active {
                    let stays_tapped = game.obj(id).def.abilities.iter().any(|a| match a {
                        AbilityDef::SimpleStaticAbility { effects, .. } => {
                            effects.iter().any(|e| matches!(e, EffectDef::DontUntapInControllersUntapStepSourceEffect))
                        }
                        _ => false,
                    }) || game.any_static(|g, src, e| {
                        matches!(e, EffectDef::DontUntapInControllersUntapStepEnchantedEffect)
                            && g.attached(src) == Some(id)
                    });
                    // `DontUntapInControllersNextUntapStepTargetEffect`: used up by this untap step
                    let zcc = game.obj(id).zcc;
                    let skipped = game.skip_next_untap.contains(&(id, zcc));
                    if skipped {
                        game.skip_next_untap.retain(|x| *x != (id, zcc));
                    }
                    let o = game.obj_mut(id);
                    if o.exerted {
                        o.exerted = false;
                    } else if !stays_tapped && !skipped {
                        untap(o);
                    }
                    o.sick = false;
                }
            }
        }
        Step::DeclareAttackers => {
            declare_attackers(game, d, active);
            if log::kept(game) {
                // `Combat.declareAttackers`
                let n = game.combat_groups.len();
                let line = if n > 0 {
                    let defender = game.opponent(active);
                    format!(
                        "{} attacks {} with {} creature{}",
                        log::player(active),
                        log::player(defender),
                        n,
                        if n == 1 { "" } else { "s" }
                    )
                } else {
                    format!("{} skips attack", log::player(active))
                };
                log::inform(game, line);
            }
            if !attackers(game).is_empty() {
                game.player_mut(active).attacked = true;
            }
            // ATTACKER_DECLARED's replacement: "you may exert this as it attacks", asked for each attacker in turn,
            // and its "when you do" trigger
            for a in attackers(game) {
                if !game.has_ability(a, &AbilityDef::ExertAbility) {
                    continue;
                }
                let q = Query::YesNo { message: format!("Exert {}?", game.obj(a).name()) };
                let yes = loop {
                    match ask(game, d, active, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if yes {
                    game.obj_mut(a).exerted = true;
                    let fired = game.triggers([a], |_, _, _, t| matches!(t, Trigger::Exerted));
                    game.pending.extend(fired);
                }
            }
            // ATTACKER_DECLARED triggers, in the order the attackers are listed
            for a in attackers(game) {
                let sources: Vec<ObjId> = game.trigger_sources();
                let fired = game.triggers(sources, |g, src, you, t| match t {
                    Trigger::EntersOrAttacks => src == a,
                    Trigger::AttacksAll { filter } => g.filter_matches(filter, you, Some(src), a),
                    _ => false,
                });
                for mut t in fired {
                    let all = t.ability.and_then(|(i, _)| game.item_def(&t).abilities[i].trigger().cloned());
                    if matches!(all, Some(Trigger::AttacksAll { .. })) && set_target_pointer(game, &t) {
                        t.targets = vec![vec![Target::Obj(a)]];
                        t.target_zcc = vec![vec![game.obj(a).zcc]];
                    }
                    game.pending.push(t);
                }
            }
            // then DECLARED_ATTACKERS, one event: "whenever this attacks" (`AttacksTriggeredAbility`), "whenever you
            // attack with N or more"
            let all = attackers(game);
            if !all.is_empty() {
                let sources: Vec<ObjId> = game.trigger_sources();
                let fired = game.triggers(sources, |g, src, you, t| match t {
                    Trigger::Attacks => all.contains(&src),
                    Trigger::AttacksAlone => all == [src],
                    Trigger::Training => {
                        all.contains(&src) && {
                            let power = g.power_toughness(src).0;
                            all.iter().any(|a| g.power_toughness(*a).0 > power)
                        }
                    }
                    Trigger::AttacksWith { min, filter } => {
                        you == active
                            && all.iter().filter(|a| g.filter_matches(filter, you, Some(src), **a)).count() >= *min
                    }
                    _ => false,
                });
                game.pending.extend(fired);
                // DEFENDER_ATTACKED: the creatures attacking the defending player (not a planeswalker)
                let at_player: Vec<(ObjId, u32)> = all
                    .iter()
                    .filter(|a| game.obj(**a).attacking_pw.is_none())
                    .map(|a| (*a, game.obj(*a).zcc))
                    .collect();
                if !at_player.is_empty() {
                    let defender = game.opponent(active);
                    for (s, item) in game.attacks_opponent.clone() {
                        if s != defender {
                            let mut item = item;
                            if let Some(dl) = item.delayed.as_mut() {
                                Arc::make_mut(dl).fixed = at_player.clone();
                            }
                            game.pending.push(item);
                        }
                    }
                }
            }
        }
        Step::DeclareBlockers => {
            declare_blockers(game, d, game.opponent(active));
            if log::kept(game) && !d.stopped() {
                // `Combat.logBlockerInfo`
                for g in game.combat_groups.clone() {
                    let a = g.attacker;
                    if game.obj(a).zone != Zone::Battlefield {
                        continue;
                    }
                    let (p, t) = game.power_toughness(a);
                    let mut line = format!("Attacker: {} ({}/{}) ", log::object(game, a), p, t);
                    let blockers = live_blockers(game, a);
                    if blockers.is_empty() {
                        line += "unblocked";
                    } else {
                        line += "blocked by ";
                        for b in blockers {
                            let (p, t) = game.power_toughness(b);
                            line += &format!("{} ({}/{}) ", log::object(game, b), p, t);
                        }
                    }
                    log::inform(game, line);
                }
            }
        }
        // "at end of combat" triggers, then the delayed ones
        Step::EndCombat => {
            let sources = game.trigger_sources();
            let fired = game.triggers(sources, |g, src, you, t| {
                matches!(t, Trigger::StepBegins { step, who, graveyard: false, suspended: false }
                    if step == "endcombat" && Game::whose(who, you, active) && g.obj(src).zone == Zone::Battlefield)
            });
            game.pending.extend(fired);
            let delayed = std::mem::take(&mut game.at_end_of_combat);
            game.pending.extend(delayed);
        }
        // "at the beginning of <step>" triggers
        Step::Upkeep | Step::Draw | Step::BeginCombat | Step::PrecombatMain | Step::PostcombatMain | Step::EndTurn => {
            if step == Step::PrecombatMain {
                add_saga_lore(game, active);
            }
            // the draw step's draw comes before its triggers
            if step == Step::Draw {
                draw(game, active, 1);
            }
            let name = match step {
                Step::Upkeep => "upkeep",
                Step::Draw => "draw",
                Step::BeginCombat => "combat",
                Step::PostcombatMain => "main2",
                Step::PrecombatMain => "main1",
                _ => "end",
            };
            let mut sources: Vec<ObjId> = game.trigger_sources();
            for p in game.players.iter() {
                sources.extend(p.graveyard.iter().copied());
            }
            sources.extend(game.exile.iter().copied().filter(|c| game.obj(*c).suspended));
            let fired = game.triggers(sources, |g, src, you, t| {
                let suspended_card = g.obj(src).zone == Zone::Exile && g.obj(src).suspended;
                matches!(t, Trigger::StepBegins { step, who, graveyard, suspended }
                    if step == name && Game::whose(who, you, active)
                        && *suspended == suspended_card
                        && *graveyard == (g.obj(src).zone == Zone::Graveyard))
                    || (matches!(t, Trigger::EntersOrUpkeep)
                        && name == "upkeep"
                        && you == active
                        && g.obj(src).zone == Zone::Battlefield)
            });
            // "at the beginning of the next end step" delayed triggers: `GameState.handleEvent` checks the delayed
            // ones before the others
            if step == Step::EndTurn {
                let delayed = std::mem::take(&mut game.at_next_end_step);
                game.pending.extend(delayed);
            }
            if step == Step::Upkeep {
                let delayed = std::mem::take(&mut game.at_next_upkeep);
                game.pending.extend(delayed);
                let (mine, rest): (Vec<_>, Vec<_>) =
                    std::mem::take(&mut game.at_first_upkeep).into_iter().partition(|i| i.controller == active);
                game.at_first_upkeep = rest;
                game.pending.extend(mine);
                let (mine, rest): (Vec<_>, Vec<_>) =
                    std::mem::take(&mut game.rebound).into_iter().partition(|(_, _, s)| *s == active);
                game.rebound = rest;
                for (card, zcc, owner) in mine {
                    let delayed = Delayed {
                        rule: "Rebound - You may cast {this} from exile without paying its mana cost.".into(),
                        sacrifice: Vec::new(),
                        warp_exile: Vec::new(),
                        effects: vec![EffectDef::ReboundCastSpellFromExileEffect],
                        source_zcc: zcc,
                        fixed: Vec::new(),
                        return_tapped: Vec::new(),
                        targets: Vec::new(),
                        condition: None,
                    };
                    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(card, owner) });
                }
            }
            // the delayed ones (the next upkeep's, rebound) before the others, as at the end step
            game.pending.extend(fired);
        }
        // the replacement choices its damage asked for (Twinflame Tyrants) come before the step's priority
        Step::FirstCombatDamage => {
            game.first_strike_step = true;
            combat_damage(game, d, true);
            settle_owed(game, d);
        }
        Step::CombatDamage => {
            combat_damage(game, d, false);
            settle_owed(game, d);
        }
        Step::Cleanup => cleanup(game, d, active),
    }
}

fn end_step(game: &mut Game, step: Step) {
    // mana empties as steps end; firebending's {R} stays until combat ends
    let in_combat = matches!(
        step,
        Step::BeginCombat
            | Step::DeclareAttackers
            | Step::DeclareBlockers
            | Step::FirstCombatDamage
            | Step::CombatDamage
    );
    for s in 0..game.seats() {
        let p = game.player_mut(s);
        let keep = if in_combat { p.combat_red.min(p.mana_pool.colored[Color::R.index()]) } else { 0 };
        p.mana_pool = Mana::of(Color::R, keep);
        if keep == 0 {
            p.mana_pool = Mana::default();
        }
        p.combat_red = keep;
        p.conditional_pool.clear();
        p.mana_stock = Mana::default();
    }
    if step == Step::EndCombat {
        game.first_strike_step = false;
        game.first_strikers.clear();
        game.combat_groups.clear();
        for id in Arc::clone(&game.battlefield).iter().copied() {
            let o = game.obj(id);
            if o.attacking.is_some() || !o.blocking.is_empty() || o.blocks > 0 || o.blocked {
                let o = game.obj_mut(id);
                o.attacking = None;
                o.attacking_pw = None;
                o.blocking.clear();
                o.blocks = 0;
                o.blocked = false;
                o.blocked_by.clear();
            }
        }
    }
}

/// Draws `n` cards, one at a time: each card drawn is a `DREW_CARD` event for "whenever … draws" triggers.
pub fn draw(game: &mut Game, seat: Seat, n: usize) {
    let before = game.player(seat).drawn;
    for _ in 0..n {
        draw_one(game, seat, &[]);
    }
    let drawn = game.player(seat).drawn.saturating_sub(before) as usize;
    if drawn > 0 && log::kept(game) {
        let line =
            format!("{} draws {} card{}", log::player(seat), log::number(drawn), if drawn > 1 { "s" } else { "" });
        log::inform(game, line);
    }
}

/// One `DRAW_CARD` event; `applied`: the replacement effects (their sources) already applied to it.
fn draw_one(game: &mut Game, seat: Seat, applied: &[ObjId]) {
    {
        // Narset, Parter of Veils: each opponent can't draw more than one card each turn
        if game.player(seat).drawn >= 1
            && game.any_static(|g, src, e| {
                matches!(e, EffectDef::NarsetParterOfVeilsEffect) && g.obj(src).controller != seat
            })
        {
            return;
        }
        // Quantum Riddler: with one or fewer cards in hand, two cards instead
        if game.player(seat).hand.len() <= 1 {
            let mut riddlers = Vec::new();
            game.any_static(|g, src, e| {
                if matches!(e, EffectDef::QuantumRiddlerReplacementEffect)
                    && g.obj(src).controller == seat
                    && !applied.contains(&src)
                {
                    riddlers.push(src);
                }
                false
            });
            // two or more: the player chooses which applies first (they all do the same)
            if riddlers.len() > 1 {
                game.owed_choices.push(seat);
            }
            if let Some(r) = riddlers.first().copied() {
                let mut applied = applied.to_vec();
                applied.push(r);
                draw_one(game, seat, &applied);
                draw_one(game, seat, &applied);
                return;
            }
        }
        match game.player(seat).library.first().copied() {
            Some(id) => {
                game.player_mut(seat).drawn += 1;
                game.move_to(id, Zone::Hand);
                game.record(GameEvent {
                    target: Some(Target::Obj(id)),
                    player: Some(seat),
                    from: Some(Zone::Library),
                    to: Some(Zone::Hand),
                    ..GameEvent::new(EventKind::Draw)
                });
                let sources: Vec<ObjId> = game.trigger_sources();
                let drawn = game.player(seat).drawn;
                let fired = game.triggers(sources, |_, _, you, t| match t {
                    Trigger::Draws { who } => Game::whose(who, you, seat),
                    Trigger::DrawsNth { n } => you == seat && drawn == *n,
                    _ => false,
                });
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
    let a = t.ability.and_then(|(i, _)| game.item_def(t).abilities.get(i));
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
            // `SkipPrioritySteps.isStepSkipped` has no setting for it (there's priority only under 514.3a)
            | Step::Cleanup
    )
}

/// `GameImpl.playPriority`: each player in turn order from the active one until all pass in a row; then the top of
/// the stack resolves and priority starts again with the active player, or the step ends.
fn priority_round(game: &mut Game, d: &mut dyn Decider, mut resume: bool) {
    let active = game.active.unwrap_or(0);
    if !resume {
        // `GameImpl.playPriority` starts the loop count again
        game.loop_count = 0;
    }
    loop {
        let mut seat = active;
        if std::mem::take(&mut resume) {
            seat = game.priority.unwrap_or(active);
        } else {
            for s in 0..game.seats() {
                game.player_mut(s).passed = false;
            }
        }
        loop {
            game.priority = Some(seat);
            while !game.player(seat).passed {
                check_state_and_triggered(game, d);
                if game.over || d.stopped() {
                    return;
                }
                traced(game, d);
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
        let source = game.stack.last().map(|i| i.card);
        resolve_top(game, d);
        if let Some(source) = source {
            check_infinite_loop(game, d, source);
            if game.over || d.stopped() {
                return;
            }
        }
        traced(game, d);
        if d.stopped() {
            return;
        }
    }
}

/// How many times in a row the players may decline to call a detected loop a draw before the engine calls it one
/// without asking (Java asks forever; a hosted game must end)
const LOOP_DECLINES: u32 = 10;

/// `GameImpl.checkInfiniteLoop`, after each resolution: the same source resolving again (one of the last four) with
/// the stack no smaller than after the last resolution, more than 15 times in one priority round with no player
/// losing life, is a loop. Each player, the source's controller first, is asked "Draw game because of infinite
/// looping?"; if all agree the game is a draw (104.4b), and one "no" starts the count again.
fn check_infinite_loop(game: &mut Game, d: &mut dyn Decider, source: ObjId) {
    if game.loop_sources.contains(&source) && game.stack.len() >= game.loop_stack {
        let lives: Vec<i32> = game.players.iter().map(|p| p.life).collect();
        match &game.loop_lives {
            // a player losing life isn't a loop (Java keeps the first list otherwise)
            Some(last) if last.len() == lives.len() => {
                if lives.iter().zip(last).any(|(now, then)| now < then) {
                    game.loop_lives = None;
                    game.loop_count = 0;
                    game.loop_declined = 0;
                }
            }
            _ => game.loop_lives = Some(lives),
        }
        game.loop_count += 1;
        if game.loop_count > 15 {
            let controller = game.obj(source).controller;
            let seats: Vec<Seat> = (0..game.seats()).map(|k| (controller + k) % game.seats()).collect();
            let forced = game.loop_declined >= LOOP_DECLINES;
            for &seat in &seats {
                if forced {
                    break;
                }
                let q = Query::YesNo { message: "Draw game because of infinite looping?".into() };
                match ask(game, d, seat, &q) {
                    Answer::Bool(true) => {
                        if log::kept(game) {
                            let line = format!(
                                "{} has confirmed that the game is a draw because of infinite looping.",
                                log::player(controller)
                            );
                            log::inform(game, line);
                        }
                    }
                    Answer::Stop => return,
                    _ => {
                        if log::kept(game) {
                            let line = format!(
                                "{} has NOT confirmed that the game is a draw because of infinite looping.",
                                log::player(controller)
                            );
                            log::inform(game, line);
                        }
                        game.loop_count = 0;
                        game.loop_declined += 1;
                        return;
                    }
                }
            }
            for seat in seats {
                if !game.player(seat).lost {
                    game.player_mut(seat).drew = true;
                    if log::kept(game) {
                        let line = format!("For {} the game is a draw.", log::player(seat));
                        log::inform(game, line);
                    }
                }
            }
            game.over = true;
        }
    } else {
        game.loop_sources.push(source);
        if game.loop_sources.len() > 4 {
            game.loop_sources.remove(0);
        }
    }
    game.loop_stack = game.stack.len();
}

/// Devourer of Destiny's delayed trigger: "at the beginning of your first upkeep, look at the top four cards of your
/// library. You may put one of those cards back on top of your library. Exile the rest."
pub fn devourer_first_upkeep(game: &mut Game, card: ObjId, seat: Seat) {
    let delayed = Delayed {
        rule: "look at the top four cards of your library. Put up to one of them back on top of your library and the \
               rest into exile."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::LookLibraryAndPickControllerEffect {
            look: 4,
            look_count: None,
            pick: 1,
            filter: Default::default(),
            picked: crate::card::PutCards::TopAny,
            looked: crate::card::PutCards::Exiled,
            up_to: true,
            optional: false,
            pick_x: false,
        }],
        source_zcc: game.obj(card).zcc,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_first_upkeep.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(card, seat) });
}

/// The opening hands' actions after the mulligans (`GameImpl.init`, 103.6), the starting player first: a card to act
/// on when there are several, then whether to (Devourer of Destiny's reveal, a leyline, Gemstone Caverns for the
/// player going second). False: the decider stopped.
pub fn opening_hand_actions(game: &mut Game, d: &mut dyn Decider, first: Seat) -> bool {
    for seat in [first, game.opponent(first)] {
        let mut cards: Vec<ObjId> = game
            .player(seat)
            .hand
            .iter()
            .copied()
            .filter(|c| {
                let def = &game.obj(*c).def;
                def.has(&AbilityDef::ChancellorAbility)
                    || def.has(&AbilityDef::LeylineAbility)
                    || seat != first && def.has(&AbilityDef::GemstoneCavernsAbility)
            })
            .collect();
        while !cards.is_empty() {
            let card = if cards.len() > 1 {
                match ask(game, d, seat, &Query::Cards { objs: cards.clone(), required: true }) {
                    Answer::Obj(o) if cards.contains(&o) => o,
                    Answer::Stop => return false,
                    _ => continue,
                }
            } else {
                cards[0]
            };
            cards.retain(|c| *c != card);
            let name = game.obj(card).def.name.clone();
            if game.obj(card).def.has(&AbilityDef::ChancellorAbility) {
                match ask(game, d, seat, &Query::YesNo { message: format!("Reveal {name}?") }) {
                    Answer::Bool(true) => devourer_first_upkeep(game, card, seat),
                    Answer::Stop => return false,
                    _ => {}
                }
            }
            // a leyline (`LeylineAbility`): begin the game with it on the battlefield
            if game.obj(card).def.has(&AbilityDef::LeylineAbility) && game.obj(card).zone == Zone::Hand {
                match ask(game, d, seat, &Query::YesNo { message: format!("Put {name} on the battlefield?") }) {
                    Answer::Bool(true) => enter_battlefield(game, d, card, seat, false),
                    Answer::Stop => return false,
                    _ => {}
                }
            }
            if game.obj(card).def.has(&AbilityDef::GemstoneCavernsAbility) && game.obj(card).zone == Zone::Hand {
                match ask(game, d, seat, &Query::YesNo { message: format!("Put {name} onto the battlefield?") }) {
                    Answer::Bool(true) => {
                        enter_battlefield(game, d, card, seat, false);
                        game.obj_mut(card).add_counters("luck", 1);
                        let hand = game.player(seat).hand.clone();
                        if !hand.is_empty() {
                            let Some(chosen) = choose_cards(game, d, seat, &hand, &hand, 1, 1, "exile") else {
                                return false;
                            };
                            for c in chosen {
                                game.move_to(c, Zone::Exile);
                            }
                        }
                    }
                    Answer::Stop => return false,
                    _ => {}
                }
            }
        }
    }
    !d.stopped()
}

/// Asks a seat a question. Like `HumanPlayer.prepareForResponse`, asking makes that seat the priority player.
fn ask(game: &mut Game, d: &mut dyn Decider, seat: Seat, q: &Query) -> Answer {
    settle_owed(game, d);
    refresh_becomes(game);
    game.priority = Some(seat);
    d.answer(game, seat, q)
}

/// Asks the questions `move_to` owes (`Game::owed_choices`); the answers change nothing.
fn settle_owed(game: &mut Game, d: &mut dyn Decider) {
    for seat in std::mem::take(&mut game.owed_choices) {
        let q = Query::Choice { message: "Choose a replacement effect to apply first".into(), choices: Vec::new() };
        game.priority = Some(seat);
        d.answer(game, seat, &q);
    }
}

/// `Decider::traced`, after the questions owed so far.
fn traced(game: &mut Game, d: &mut dyn Decider) {
    settle_owed(game, d);
    refresh_becomes(game);
    d.traced(game);
}

fn pass(game: &mut Game, seat: Seat) {
    game.short_lki.clear();
    game.player_mut(seat).passed = true;
}

/// `HumanPlayer.passWithManaPoolCheck`: passing with mana in the pool asks first. False: the player didn't pass.
fn pass_with_mana_pool_check(game: &mut Game, d: &mut dyn Decider, seat: Seat) -> bool {
    if game.stack.is_empty() && game.player(seat).mana_pool.total() > 0 && !game.player(seat).no_pool_check {
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
    if game.stack.is_empty()
        && !stops_in(step)
        && !game.player(seat).every_priority
        && pass_with_mana_pool_check(game, d, seat)
    {
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
                // a permanent clicked for its mana ability works too (`HumanPlayer.activateAbility`)
                if playable.contains(&id) || has_mana_ability_now(game, seat, id) {
                    let p = game.player_mut(seat);
                    p.mana_stock = if p.no_stock { Mana::default() } else { p.mana_pool };
                    play(game, d, seat, id);
                }
                // played (priority again) or not playable (asked again): both come back to the priority loop
                return;
            }
            // a Room's half clicked in hand: the card (its half is asked again)
            Answer::Ability(id, _) if !game.obj(id).def.room.is_empty() && playable.contains(&id) => {
                let p = game.player_mut(seat);
                p.mana_stock = if p.no_stock { Mana::default() } else { p.mana_pool };
                play(game, d, seat, id);
                return;
            }
            _ => return,
        }
    }
}

/// Whether `id` is a permanent of `seat`'s with a mana ability they can activate now (conditional ones too).
fn has_mana_ability_now(game: &Game, seat: Seat, id: ObjId) -> bool {
    if !game.battlefield.contains(&id) {
        return false;
    }
    let paying = game.paying_for.get();
    game.paying_for.set(Some(ANY_SPELL));
    let any = mana_sources_with_kinds(game, seat).iter().any(|(s, a)| *s == id && !a.is_empty());
    game.paying_for.set(paying);
    any
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
        } else if (o.def.room.is_empty() && castable(game, seat, *id, &o.def))
            || o.def.room.iter().enumerate().any(|(k, h)| (!o.def.aftermath || k == 0) && castable(game, seat, *id, h))
            || o.def.adventure.as_ref().is_some_and(|a| castable(game, seat, *id, a))
            || spectacle(game, seat, *id).is_some()
            || warp(game, seat, *id).is_some()
            || cleave(game, seat, *id).is_some()
            || overload(game, seat, *id).is_some()
            || blitz(game, seat, *id).is_some()
            || foretell(game, seat, *id).is_some()
            || plot(game, seat, *id).is_some()
            || suspend(game, seat, *id).is_some()
            || !activatable(game, seat, *id).is_empty()
            || o.def.back.as_ref().is_some_and(|b| b.is(CardType::Land) && can_play_land(game, seat))
        {
            out.push(*id);
        }
    }
    // flashback, from the graveyard (Weathered Runestone: players can't cast spells from graveyards)
    let runestone = game.any_static(|_, _, e| matches!(e, EffectDef::WeatheredRunestoneEffect2));
    let lands = game.any_static(|g, src, e| {
        matches!(e, EffectDef::PlayLandsFromGraveyardControllerEffect) && g.obj(src).controller == seat
    }) && can_play_land(game, seat);
    for id in p.graveyard.iter() {
        let def = &game.obj(*id).def;
        if def.is(CardType::Land) {
            // played from it (Crucible of Worlds), or a graveyard ability of its own (Drownyard Temple)
            if lands || !activatable(game, seat, *id).is_empty() {
                out.push(*id);
            }
            continue;
        }
        if runestone {
            continue;
        }
        if game.flashback_def(*id).is_some_and(|f| castable(game, seat, *id, &f))
            || (game.obj(*id).may_play.is_some_and(|(p, t)| p == seat && game.turn <= t)
                && castable(game, seat, *id, def))
            || def.escape().is_some_and(|f| castable(game, seat, *id, &f))
            || blitz(game, seat, *id).is_some()
            || !activatable(game, seat, *id).is_empty()
        {
            out.push(*id);
        }
    }
    // a card on an adventure: its creature, from exile; a card you may play from exile
    for id in game.exile.iter() {
        let o = game.obj(*id);
        if o.on_adventure
            && o.owner == seat
            && if o.is(CardType::Land) { can_play_land(game, seat) } else { castable(game, seat, *id, &o.def) }
        {
            out.push(*id);
        } else if o.foretold_turn.is_some() || o.plotted_turn.is_some() {
            if exile_cast(game, seat, *id).is_some() {
                out.push(*id);
            }
        } else if valgavoth_play(game, seat, *id) {
            if if o.is(CardType::Land) {
                can_play_land(game, seat)
            } else {
                castable(game, seat, *id, &valgavoth_def(&o.def))
            } {
                out.push(*id);
            }
        } else if (o
            .may_play
            .is_some_and(|(p, t)| p == seat && game.turn <= t && !(o.cast_only && o.is(CardType::Land)))
            || o.warp_from.is_some_and(|t| o.owner == seat && game.turn >= t))
            && if o.is(CardType::Land) {
                can_play_land(game, seat)
            } else {
                castable(game, seat, *id, &o.def)
                    || o.play_free
                        && castable(game, seat, *id, &CardDef { cost: ManaCost::default(), ..(*o.def).clone() })
                    || o.def.adventure.as_ref().is_some_and(|a| castable(game, seat, *id, a))
                    || spectacle(game, seat, *id).is_some()
            }
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

/// How the client groups a playable ability (`PlayableObjectStats`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayKind {
    /// a `BasicManaAbility`
    BasicMana,
    /// a `PlayLandAbility`
    Land,
    /// a spell's own `SpellAbility`
    Cast,
    Other,
    /// a Room card's half, cast on its own (`h0`, `h1`): its index is the half's
    RoomHalf,
    /// an adventurer card's adventure (`a0`, its spell card `f1`)
    Adventure,
    /// a plotted card cast from exile, by its `PlotAbility`
    Plotted,
    /// a foretold card cast from exile for its foretell cost (`ForetellCostAbility`)
    Foretold,
}

/// What `getPlayableObjects` lists for the client: each object with an ability `seat` can play now, as the
/// ability's index in its card's abilities and its group, in the order Java finds them (hand, graveyard, exile, then
/// the battlefield)
pub fn playable_abilities(game: &Game, seat: Seat) -> Vec<(ObjId, usize, PlayKind, Option<u8>)> {
    let mut out = Vec::new();
    // `shouldSkipGettingPlayable`: nothing while attackers are declared
    if game.step_start && game.step == Some(Step::DeclareAttackers) {
        return out;
    }
    let index = |def: &CardDef, a: &AbilityDef| def.abilities.iter().position(|x| x == a);
    for id in playable(game, seat) {
        let o = game.obj(id);
        let def = &o.def;
        match o.zone {
            Zone::Hand | Zone::Graveyard | Zone::Exile => {
                // a double-faced card's faces: each its half too (`f0`, `f1`), the back's abilities after the front's
                let mdfc = def.back.as_ref().filter(|_| o.zone == Zone::Hand);
                let face = (mdfc.is_some() || def.transform_back.is_some()).then_some(0);
                if let Some(back) = mdfc {
                    if back.is(CardType::Land) && can_play_land(game, seat) {
                        if let Some(k) = index(back, &AbilityDef::PlayLandAbility) {
                            out.push((id, def.abilities.len() + k, PlayKind::Land, Some(1)));
                        }
                    }
                }
                if o.is(CardType::Land) {
                    // from a graveyard only with Crucible of Worlds' effect
                    let from_graveyard = o.zone != Zone::Graveyard
                        || game.any_static(|g, src, e| {
                            matches!(e, EffectDef::PlayLandsFromGraveyardControllerEffect)
                                && g.obj(src).controller == seat
                        });
                    if let Some(k) =
                        index(def, &AbilityDef::PlayLandAbility).filter(|_| from_graveyard && can_play_land(game, seat))
                    {
                        out.push((id, k, PlayKind::Land, face));
                    }
                } else {
                    let spell = index(def, &AbilityDef::SpellAbility);
                    let normal = match o.zone {
                        Zone::Hand => castable(game, seat, id, def),
                        Zone::Graveyard => {
                            o.may_play.is_some_and(|(p, t)| p == seat && game.turn <= t)
                                && castable(game, seat, id, def)
                        }
                        // a foretold card is cast from exile only for its foretell cost
                        Zone::Exile if o.foretold_turn.is_some() => false,
                        _ => castable(game, seat, id, def),
                    };
                    if o.zone == Zone::Exile && o.plotted_turn.is_some() && exile_cast(game, seat, id).is_some() {
                        // (`PlotAbility` gives the card its cast from exile after its own abilities)
                        if def.plot().is_some() {
                            out.push((id, def.abilities.len(), PlayKind::Plotted, None));
                        }
                    }
                    // (`ForetellAbility` gives the card its cast from exile after its own abilities)
                    if o.zone == Zone::Exile
                        && o.foretold_turn.is_some()
                        && exile_cast(game, seat, id).is_some()
                        && def.foretold().is_some()
                    {
                        out.push((id, def.abilities.len(), PlayKind::Foretold, None));
                    }
                    if let Some(adventure) = &def.adventure {
                        if (o.zone == Zone::Hand || (o.zone == Zone::Exile && !o.on_adventure))
                            && castable(game, seat, id, adventure)
                        {
                            out.push((id, 0, PlayKind::Adventure, Some(1)));
                        }
                    }
                    if o.zone == Zone::Hand {
                        for (h, half) in def.room.iter().enumerate() {
                            if (!def.aftermath || h == 0) && castable(game, seat, id, half) {
                                out.push((id, h, PlayKind::RoomHalf, None));
                            }
                        }
                    }
                    // (a double-faced card's front is cast with a `SpellAbility` that isn't the `BASE` one)
                    if let (true, Some(k)) = (normal, spell) {
                        out.push((id, k, if face.is_some() { PlayKind::Other } else { PlayKind::Cast }, face));
                    }
                    let alternatives = [
                        spectacle(game, seat, id).map(|x| x.0),
                        warp(game, seat, id).map(|x| x.0),
                        cleave(game, seat, id).map(|x| x.0),
                        overload(game, seat, id).map(|x| x.0),
                        blitz(game, seat, id).map(|x| x.0),
                        foretell(game, seat, id),
                        plot(game, seat, id).map(|x| x.0),
                        suspend(game, seat, id).map(|x| x.0),
                    ];
                    out.extend(alternatives.into_iter().flatten().map(|k| (id, k, PlayKind::Other, None)));
                    // an aftermath card's right half (`h1`)
                    if o.zone == Zone::Graveyard
                        && def.aftermath
                        && game.flashback_def(id).is_some_and(|f| castable(game, seat, id, &f))
                    {
                        out.push((id, 1, PlayKind::RoomHalf, None));
                    }
                    if o.zone == Zone::Graveyard {
                        let flashback =
                            def.abilities.iter().position(|a| matches!(a, AbilityDef::FlashbackAbility { .. }));
                        if let Some(k) =
                            flashback.filter(|_| game.flashback_def(id).is_some_and(|f| castable(game, seat, id, &f)))
                        {
                            out.push((id, k, PlayKind::Other, None));
                        }
                        let escape = def.abilities.iter().position(|a| matches!(a, AbilityDef::EscapeAbility { .. }));
                        if let Some(k) = escape.filter(|_| def.escape().is_some_and(|f| castable(game, seat, id, &f))) {
                            out.push((id, k, PlayKind::Other, None));
                        }
                    }
                }
                out.extend(activatable(game, seat, id).into_iter().map(|k| (id, k, PlayKind::Other, None)));
            }
            _ => {}
        }
    }
    // the battlefield: mana abilities (conditional ones too), then the others, per permanent
    let paying = game.paying_for.get();
    game.paying_for.set(Some(ANY_SPELL));
    let mana = mana_sources_with_kinds(game, seat);
    game.paying_for.set(paying);
    for id in game.battlefield.iter() {
        let mut abilities: Vec<(usize, PlayKind)> = mana
            .iter()
            .filter(|(s, _)| s == id)
            .flat_map(|(_, a)| a.iter())
            .map(|(k, _, _)| {
                // a `BasicManaAbility`: one of the six, painlands' too
                let basic = matches!(
                    game.obj(*id).def.abilities.get(*k),
                    Some(
                        AbilityDef::WhiteManaAbility { .. }
                            | AbilityDef::BlueManaAbility { .. }
                            | AbilityDef::BlackManaAbility { .. }
                            | AbilityDef::RedManaAbility { .. }
                            | AbilityDef::GreenManaAbility { .. }
                            | AbilityDef::ColorlessManaAbility { .. }
                    )
                );
                (*k, if basic { PlayKind::BasicMana } else { PlayKind::Other })
            })
            .collect();
        abilities.extend(activatable(game, seat, *id).into_iter().map(|k| (k, PlayKind::Other)));
        abilities.sort_by_key(|(k, _)| *k);
        abilities.dedup();
        // a modal double-faced card's permanent: its card too (`f0`)
        let o = game.obj(*id);
        let face = (o.front.is_some() || o.def.back.is_some()).then_some(0);
        out.extend(abilities.into_iter().map(|(k, kind)| (*id, k, kind, face)));
    }
    out
}

/// A permanent's activated abilities (not mana abilities) its controller can activate now, by index.
fn activatable(game: &Game, seat: Seat, id: ObjId) -> Vec<usize> {
    let o = game.obj(id);
    // a card in a graveyard is its owner's
    if (o.zone == Zone::Graveyard && o.owner != seat) || (o.zone != Zone::Graveyard && o.controller != seat) {
        return Vec::new();
    }
    if activations_forbidden(game, id) {
        return Vec::new();
    }
    // Petrified Hamlet: (not mana abilities, which aren't asked here)
    let name = &o.def.name;
    if game.any_static(|g, src, e| {
        matches!(e, EffectDef::PetrifiedHamletCostEffect | EffectDef::PithingNeedleEffect)
            && g.obj(src).chosen_name.as_ref() == Some(name)
    }) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, a) in o.def.abilities.iter().enumerate() {
        // a Room's locked door: unlocked as a sorcery, paying its cost (the playable check counts conditional mana)
        if let AbilityDef::RoomUnlockAbility { cost, .. } = a {
            if o.zone == Zone::Battlefield
                && sorcery_speed(game, seat)
                && ManaCost::parse(cost).is_ok_and(|c| {
                    let paying = game.paying_for.get();
                    game.paying_for.set(Some(ANY_SPELL));
                    let ok = can_pay(game, seat, &c);
                    game.paying_for.set(paying);
                    ok
                })
            {
                out.push(i);
            }
            continue;
        }
        let Some((_, _, targets, costs, sorcery)) = a.activated() else { continue };
        // cycling works from hand, everything else on the battlefield
        if a.from_hand() != (o.zone == Zone::Hand) || a.from_graveyard() != (o.zone == Zone::Graveyard) {
            continue;
        }
        if let AbilityDef::GraveyardActivatedAbility { condition, .. } = a {
            if !game.condition_holds(condition.as_ref(), seat, id) {
                continue;
            }
        }
        // The Wandering Emperor: her loyalty abilities at instant speed the turn she entered
        let flash = matches!(a, AbilityDef::LoyaltyAbility { .. })
            && o.entered_turn == game.turn
            && o.def.abilities.iter().any(|b| match b {
                AbilityDef::SimpleStaticAbility { effects, .. } => {
                    effects.iter().any(|e| matches!(e, EffectDef::TheWanderingEmperorEffect))
                }
                _ => false,
            });
        if sorcery && !flash && !sorcery_speed(game, seat) {
            continue;
        }
        if let AbilityDef::LimitedTimesPerTurnActivatedAbility { limit, condition, .. } = a {
            let used = game.activated_this_turn.iter().filter(|x| **x == (id, i)).count() as u32;
            if used >= *limit || !game.condition_holds(condition.as_ref(), seat, id) {
                continue;
            }
        }
        if matches!(a, AbilityDef::PowerUpAbility { .. }) && o.powered_up {
            continue;
        }
        if matches!(a, AbilityDef::ClassLevelAbility { level, .. } if o.class_level + 1 != *level) {
            continue;
        }
        let payable = costs.iter().all(|c| match c {
            CostDef::ManaCost { .. } => {
                // the playable check counts conditional mana here too (the payment then enforces it)
                let paying = game.paying_for.get();
                game.paying_for.set(Some(ANY_SPELL));
                let ok = can_pay(game, seat, &adjusted_mana_cost(game, seat, id, a, costs));
                game.paying_for.set(paying);
                ok
            }
            _ => can_pay_cost(game, seat, id, c),
        });
        let targeted = targets.iter().all(|t| target_possible(game, seat, t, Some(id)));
        if payable && targeted {
            out.push(i);
        }
    }
    out
}

/// `UnblockedPredicate`: your attacking creatures that weren't blocked, once blockers are declared.
fn unblocked_attackers(game: &Game, seat: Seat) -> Vec<ObjId> {
    if !matches!(
        game.step,
        Some(Step::DeclareBlockers | Step::FirstCombatDamage | Step::CombatDamage | Step::EndCombat)
    ) {
        return Vec::new();
    }
    game.battlefield
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.controller == seat && o.attacking.is_some() && !o.blocked && game.has_type(*c, CardType::Creature)
        })
        .collect()
}

/// Whether a cost of `id`'s ability (or spell) can be paid now (`Cost.canPay`).
fn can_pay_cost(game: &Game, seat: Seat, id: ObjId, c: &CostDef) -> bool {
    let o = game.obj(id);
    match c {
        CostDef::HarmonizeCost | CostDef::ExertSourceCost => true,
        CostDef::ManaCost { cost } => ManaCost::parse(cost).map(|m| can_pay(game, seat, &m)).unwrap_or(false),
        CostDef::TapSourceCost => {
            let sick =
                game.has_type(id, CardType::Creature) && o.sick && !game.has_ability(id, &AbilityDef::HasteAbility);
            !(o.tapped || sick)
        }
        CostDef::SacrificeSourceCost | CostDef::ExileSourceCost => true,
        CostDef::ExileSourceFromGraveCost => o.zone == Zone::Graveyard,
        CostDef::ExileFromGraveCost { amount } => {
            game.player(seat).graveyard.iter().filter(|c| **c != id).count() >= *amount
        }
        CostDef::ReturnAttackerToHandTargetCost => !unblocked_attackers(game, seat).is_empty(),
        CostDef::PayLoyaltyCost { amount } => !o.loyalty_used && o.counter("loyalty") + amount >= 0,
        CostDef::RemoveCountersSourceCost { counter } => o.counter(&counter.name) >= counter.amount,
        CostDef::PutCountersSourceCost { .. } => true,
        // `canChooseOrAlreadyChosen`: a permanent to choose, with counters or not
        CostDef::RemoveCountersFromAmongCost { filter, .. } => {
            matching(game, filter, seat, id).iter().any(|p| game.obj(*p).controller == seat)
        }
        // you can only sacrifice your own permanents
        CostDef::SacrificeTargetCost { filter, amount } => {
            matching(game, filter, seat, id).iter().filter(|p| game.obj(**p).controller == seat).count() >= *amount
        }
        CostDef::TapTargetCost { filter, amount } => {
            matching(game, filter, seat, id).iter().filter(|p| game.obj(**p).controller == seat).count() >= *amount
        }
        CostDef::SacrificeXTargetCost { .. } | CostDef::TapXTargetCost { .. } => true,
        // `CrewCost.canPay` looks without a source, so "another" doesn't leave out the vehicle itself (crewed, it
        // counts its own power)
        CostDef::CrewCost { value } => {
            let mut sum: i32 = crew_candidates(game, seat, id).iter().map(|c| game.power_toughness(*c).0.max(0)).sum();
            if !o.tapped && game.has_type(id, CardType::Creature) {
                sum += game.power_toughness(id).0.max(0);
            }
            sum >= *value
        }
        CostDef::DiscardCardCost { filter, amount } => {
            game.player(seat).hand.iter().filter(|c| game.filter_matches(filter, seat, Some(id), **c)).count()
                >= *amount
        }
        CostDef::PayLifeCost { amount } => game.player(seat).life >= *amount,
        CostDef::PayEnergyCost { amount } => energy(game, seat) >= *amount,
        CostDef::RevealTargetFromHandCost { filter, min, .. } => {
            game.player(seat)
                .hand
                .iter()
                .filter(|c| **c != id && game.filter_matches(filter, seat, Some(id), **c))
                .count()
                >= *min
        }
        // (while it's checked as playable, a card in hand counts itself: Solitude's evoke, Force of Negation's
        // alternative cost)
        CostDef::ExileFromHandCost { filter } => {
            game.player(seat).hand.iter().any(|c| game.filter_matches(filter, seat, Some(id), *c))
        }
        CostDef::BeholdCost { subtype } => {
            let (perms, cards) = beholdable(game, seat, subtype);
            !perms.is_empty() || !cards.is_empty()
        }
        CostDef::DiscardSourceCost => o.zone == Zone::Hand,
        // either one (`OrCost`)
        CostDef::OrCost { costs } => costs.iter().any(|c| or_cost_payable(game, seat, id, c)),
        CostDef::ForageCost => can_pay_cost(game, seat, id, &forage_cost()),
    }
}

/// `ForageCost`: an `OrCost` of exiling three cards from your graveyard and sacrificing a Food.
fn forage_cost() -> CostDef {
    CostDef::OrCost {
        costs: vec![
            CostDef::ExileFromGraveCost { amount: 3 },
            CostDef::SacrificeTargetCost {
                filter: crate::card::Filter(vec![Pred::Subtype { s: "Food".into() }]),
                amount: 1,
            },
        ],
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
    // (in Java a spell being cast is on the stack while its costs are paid and the object resolving until it's done:
    // not yet or no longer on ours)
    game.active == Some(seat)
        && game.step.map(|s| s.is_main()).unwrap_or(false)
        && game.stack.is_empty()
        && game.unpaid.is_none()
        && game.resolving.is_none()
}

fn can_play_land(game: &Game, seat: Seat) -> bool {
    sorcery_speed(game, seat) && game.player(seat).lands_played < lands_per_turn(game, seat)
}

/// The lands a player may play this turn (`Player.getLandsPerTurn`)
pub fn lands_per_turn(game: &Game, seat: Seat) -> u32 {
    let mut extra = 0;
    game.any_static(|g, src, e| {
        if let EffectDef::PlayAdditionalLandsControllerEffect { amount } = e {
            if g.obj(src).controller == seat {
                extra += amount;
            }
        }
        false
    });
    1 + extra
}

/// While checking what's playable, harmonize counts the most power among untapped creatures off the cost
fn harmonized(game: &Game, seat: Seat, def: &CardDef, mut cost: ManaCost) -> ManaCost {
    if def.spell.costs.contains(&CostDef::HarmonizeCost) {
        let most = untapped_creatures(game, seat).iter().map(|c| game.power_toughness(*c).0).max().unwrap_or(0);
        cost.generic = cost.generic.saturating_sub(most.max(0) as u32);
    }
    cost
}

/// Whether the card (or its adventure, `def`) can be cast now: timing, the one-spell limit, its costs, its targets.
/// `paying_for` while checking what's playable: conditional mana counts for any spell
const ANY_SPELL: ObjId = ObjId(u32::MAX);

fn castable(game: &Game, seat: Seat, id: ObjId, def: &CardDef) -> bool {
    (game.any_time.get()
        || def.is(CardType::Instant)
        || def.has(&AbilityDef::FlashAbility)
        || sorcery_speed(game, seat)
        || {
            game.any_static(|g, src, e| match e {
                EffectDef::CastAsThoughItHadFlashAllEffect { filter } => {
                    g.obj(src).controller == seat && g.filter_matches(filter, seat, Some(src), id)
                }
                _ => false,
            })
        }
        || game
            .flash_until_next_turn
            .iter()
            .any(|(s, filter, src)| *s == seat && game.filter_matches(filter, seat, Some(*src), id)))
        // Teferi, Time Raveler: an opponent's spells only when they could cast a sorcery (`canPlaySorcery`, also for
        // spells cast while an effect resolves)
        && (sorcery_speed(game, seat)
            || !game.any_static(|g, src, e| {
                matches!(e, EffectDef::TeferiTimeRavelerEffect) && g.obj(src).controller != seat
            }))
        && !cant_cast_more(game, seat)
        && (game.player(seat).no_noncreature_turn != Some(game.turn) || def.is(CardType::Creature))
        && {
            // Java's playable check counts conditional mana for any spell (the payment then enforces it)
            game.paying_for.set(Some(ANY_SPELL));
            let pays = |cost: ManaCost| {
                can_pay_extra(
                    game,
                    seat,
                    &exile_from_hand_reduced(
                        game,
                        seat,
                        id,
                        def,
                        harmonized(game, seat, def, modified_cost_of(game, seat, id, def, cost)),
                    ),
                    alternate_mana(game, seat, id, def),
                )
            };
            // an alternative cost counts too (`findActivatedAbilityFromAlternativeSourceCost`): evoke
            let evoke = def.abilities.iter().find_map(|a| match a {
                AbilityDef::EvokeAbility { cost, costs, .. }
                    if def.printed_cost.is_none() && costs.iter().all(|c| can_pay_cost(game, seat, id, c)) =>
                {
                    if cost.is_empty() {
                        Some(ManaCost::default())
                    } else {
                        ManaCost::parse(cost).ok()
                    }
                }
                _ => None,
            });
            // and one that isn't mana while its condition holds (Force of Negation)
            let alternative = def.printed_cost.is_none() && alternative_cost(game, seat, id, def).is_some();
            let ok = pays(def.cost.clone()) || evoke.is_some_and(pays) || alternative;
            game.paying_for.set(None);
            ok
        }
        && def.spell.costs.iter().all(|c| can_pay_cost(game, seat, id, c))
        && has_targets(game, seat, id, def)
}

/// `AlternativeCostSourceAbility`: the costs paid instead of the mana cost, if its condition holds and they can be
/// paid
fn alternative_cost(game: &Game, seat: Seat, id: ObjId, def: &CardDef) -> Option<(Vec<CostDef>, String)> {
    def.abilities.iter().find_map(|a| match a {
        AbilityDef::AlternativeCostSourceAbility { costs, condition, rule } => {
            (game.condition_holds(condition.as_ref(), seat, id)
                && costs.iter().all(|c| can_pay_cost(game, seat, id, c)))
            .then(|| (costs.clone(), rule.clone()))
        }
        _ => None,
    })
}

/// `ExileCardsFromHandAdjuster` while checking what's playable: {4} off for each card of the filter in hand
fn exile_from_hand_reduced(game: &Game, seat: Seat, id: ObjId, def: &CardDef, mut cost: ManaCost) -> ManaCost {
    for a in &def.abilities {
        if let AbilityDef::ExileCardsFromHandAdjusterAbility { filter, .. } = a {
            // (the card itself counts, still in hand)
            let n = game.player(seat).hand.iter().filter(|c| game.filter_matches(filter, seat, Some(id), **c)).count();
            cost.generic = cost.generic.saturating_sub(4 * n as u32);
        }
    }
    // Rottenmouth Viper, while checking what's playable: {1} less for each permanent it could sacrifice
    if let Some(n) = sacrifice_x_reduction(game, seat, id, def) {
        cost.generic = cost.generic.saturating_sub(n as u32);
    }
    cost
}

/// A `SacrificeXTargetCost` that reduces the spell's cost (Rottenmouth Viper): how many permanents it can sacrifice
fn sacrifice_x_reduction(game: &Game, seat: Seat, id: ObjId, def: &CardDef) -> Option<usize> {
    let reduces = def.abilities.iter().any(|a| {
        matches!(a, AbilityDef::SimpleStaticAbility { effects, .. }
            if effects.contains(&EffectDef::RottenmouthViperCostReductionEffect))
    });
    def.spell.costs.iter().find_map(|c| match c {
        CostDef::SacrificeXTargetCost { filter } if reduces => {
            Some(matching(game, filter, seat, id).iter().filter(|p| game.obj(**p).controller == seat).count())
        }
        _ => None,
    })
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

/// Casts a card for its blitz costs (the whole card comes back as it leaves the stack)
fn cast_blitzed(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, def: CardDef) {
    let before = game.clone();
    let full = game.obj(id).def.clone();
    let o = game.obj_mut(id);
    o.adventurer = Some(full);
    o.def = Arc::new(def);
    if !cast(game, d, seat, id) {
        *game = before;
        return;
    }
    game.obj_mut(id).blitzed = true;
}

/// Blitz: the card's ability index and the card with its blitz costs, if it can be cast that way: from hand, or from
/// the graveyard with Tenacious Underdog's ability.
fn blitz(game: &Game, seat: Seat, id: ObjId) -> Option<(usize, CardDef)> {
    let o = game.obj(id);
    let from_graveyard = o.zone == Zone::Graveyard
        && o.def.abilities.iter().any(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => {
                effects.iter().any(|e| matches!(e, EffectDef::TenaciousUnderdogEffect))
            }
            _ => false,
        });
    if o.zone != Zone::Hand && !from_graveyard {
        return None;
    }
    let (k, def) = o.def.blitz()?;
    castable(game, seat, id, &def).then_some((k, def))
}

/// Foretell, a special action: the ability's index, if the card is in hand during its owner's turn with {2} to pay
fn foretell(game: &Game, seat: Seat, id: ObjId) -> Option<usize> {
    let o = game.obj(id);
    if o.zone != Zone::Hand || game.active != Some(seat) {
        return None;
    }
    let (k, _) = o.def.foretold()?;
    can_pay(game, seat, &ManaCost { generic: 2, ..ManaCost::default() }).then_some(k)
}

/// Plot, a special action: the ability's index and cost, if the card is in hand as a sorcery with the cost to pay
fn plot(game: &Game, seat: Seat, id: ObjId) -> Option<(usize, ManaCost)> {
    let o = game.obj(id);
    if o.zone != Zone::Hand || o.owner != seat || !sorcery_speed(game, seat) {
        return None;
    }
    let (k, cost) = o.def.plot()?;
    // counted like a spell's cost: conditional mana counts too
    game.paying_for.set(Some(ANY_SPELL));
    let ok = can_pay(game, seat, &cost);
    game.paying_for.set(None);
    ok.then_some((k, cost))
}

/// Suspend, a special action: the ability's index, time counters and cost, if the card is in hand when its spell
/// could be cast, with the cost to pay
fn suspend(game: &Game, seat: Seat, id: ObjId) -> Option<(usize, i32, ManaCost)> {
    let o = game.obj(id);
    if o.zone != Zone::Hand || o.owner != seat {
        return None;
    }
    let (k, time, cost) = o.def.suspend()?;
    let instant_speed = o.def.is(CardType::Instant) || o.def.has(&AbilityDef::FlashAbility);
    if !instant_speed && !sorcery_speed(game, seat) {
        return None;
    }
    game.paying_for.set(Some(ANY_SPELL));
    let ok = can_pay(game, seat, &cost);
    game.paying_for.set(None);
    ok.then_some((k, time, cost))
}

/// A foretold card cast for its foretell cost, or a plotted one cast free as a sorcery, on a later turn: the card
/// as it's cast, and whether it's free
fn exile_cast(game: &Game, seat: Seat, id: ObjId) -> Option<(CardDef, bool)> {
    let o = game.obj(id);
    if o.zone != Zone::Exile || o.owner != seat {
        return None;
    }
    if o.plotted_turn.is_some_and(|t| t != game.turn) && sorcery_speed(game, seat) {
        let def = CardDef { cost: ManaCost::default(), ..(*o.def).clone() };
        return castable(game, seat, id, &def).then_some((def, true));
    }
    if o.foretold_turn.is_some_and(|t| t != game.turn) {
        let (_, def) = o.def.foretold()?;
        return castable(game, seat, id, &def).then_some((def, false));
    }
    None
}

/// Valgavoth: during your turn, you may play the cards exiled with a Valgavoth of yours
fn valgavoth_play(game: &Game, seat: Seat, id: ObjId) -> bool {
    game.obj(id).zone == Zone::Exile
        && game.active == Some(seat)
        && game.exiled_for_source.iter().any(|(src, zcc, c)| {
            let s = game.obj(*src);
            *c == id
                && s.zone == Zone::Battlefield
                && s.zcc == *zcc
                && s.controller == seat
                && s.def.abilities.iter().any(|a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => {
                        effects.iter().any(|e| matches!(e, EffectDef::ValgavothTerrorEaterPlayEffect))
                    }
                    _ => false,
                })
        })
}

/// A spell cast with Valgavoth: life equal to its mana value rather than its mana cost
fn valgavoth_def(def: &CardDef) -> CardDef {
    let mut spell = def.spell.clone();
    spell.costs.insert(0, CostDef::PayLifeCost { amount: def.printed().mana_value() as i32 });
    CardDef { cost: ManaCost::default(), printed_cost: Some(def.printed().clone()), spell, ..def.clone() }
}

/// Cleave: the card's ability index and the card with its cleave cost and effects, if it can be cast that way from
/// hand.
fn cleave(game: &Game, seat: Seat, id: ObjId) -> Option<(usize, CardDef)> {
    if game.obj(id).zone != Zone::Hand {
        return None;
    }
    let (k, def) = game.obj(id).def.cleave()?;
    castable(game, seat, id, &def).then_some((k, def))
}

/// Overload: the card's ability index and the card cast for its overload cost, from hand, if it can be cast that way.
fn overload(game: &Game, seat: Seat, id: ObjId) -> Option<(usize, CardDef)> {
    if game.obj(id).zone != Zone::Hand {
        return None;
    }
    let (k, def) = game.obj(id).def.overload()?;
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
fn has_targets(game: &Game, seat: Seat, id: ObjId, def: &CardDef) -> bool {
    let spell = &def.spell;
    let all = |targets: &[TargetDef]| targets.iter().all(|t| target_possible(game, seat, t, Some(id)));
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
        && ([CardType::Artifact, CardType::Creature, CardType::Planeswalker].iter().any(|t| game.has_type(id, *t))
            && game.any_static(|_, _, e| matches!(e, EffectDef::ClarionConquerorEffect))
            || game.has_ability(id, &AbilityDef::CantActivateAbilitiesAbility))
}

thread_local! {
    /// set while checking whether a mana ability's mana cost can be paid (its own check leaves such abilities out)
    static CHECKING_MANA_COST: std::cell::RefCell<Vec<ObjId>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A permanent's mana abilities: index, mana and kind
type ManaAbilities = Vec<(usize, Mana, ManaKind)>;

/// Untapped permanents with a mana ability their controller can activate, and those abilities.
fn mana_sources_with_kinds(game: &Game, seat: Seat) -> Vec<(ObjId, ManaAbilities)> {
    let mut out = Vec::new();
    for id in game.battlefield.iter() {
        let o = game.obj(*id);
        if o.controller != seat
            || activations_forbidden(game, *id)
            || CHECKING_MANA_COST.with(|f| f.borrow().contains(id))
        {
            continue;
        }
        // an ability that taps it needs it untapped, and a creature's not summoning sick
        let can_tap = !(o.tapped
            || game.has_type(*id, CardType::Creature) && o.sick && !game.has_ability(*id, &AbilityDef::HasteAbility));
        let abilities: Vec<(usize, Mana, ManaKind)> = o
            .def
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, a)| can_tap || (a.mana().is_some() && !a.taps_source()))
            .filter(|(_, a)| a.mana_condition().is_none_or(|c| game.condition_holds(Some(c), seat, *id)))
            .filter(|(_, a)| match a {
                AbilityDef::ConditionalManaAbility { land_abilities: true, .. } => game
                    .paying_for
                    .get()
                    .is_some_and(|s| s == ANY_SPELL || (game.paying_ability.get() && game.has_type(s, CardType::Land))),
                AbilityDef::ConditionalManaAbility {
                    filter,
                    chosen_type,
                    creature_abilities,
                    filter_abilities,
                    ..
                } => game.paying_for.get().is_some_and(|s| {
                    s == ANY_SPELL
                        || *creature_abilities && game.paying_ability.get() && game.has_type(s, CardType::Creature)
                        || (*filter_abilities || !game.paying_ability.get())
                            && game.filter_matches(filter, seat, Some(*id), s)
                            && (!*chosen_type || o.chosen_type.as_ref().is_some_and(|t| game.has_subtype(s, t)))
                }),
                _ => true,
            })
            .filter(|(_, a)| {
                a.mana_costs().iter().all(|c| match c {
                    // `canPay` counts the source itself, still untapped
                    CostDef::TapTargetCost { filter, amount } => {
                        mana_tap_targets(game, seat, *id, filter).len()
                            + usize::from(game.filter_matches(filter, seat, Some(*id), *id))
                            >= *amount
                    }
                    CostDef::PayLifeCost { amount } => game.player(seat).life >= *amount,
                    CostDef::PayEnergyCost { amount } => energy(game, seat) >= *amount,
                    CostDef::SacrificeTargetCost { .. } | CostDef::RemoveCountersSourceCost { .. } => {
                        can_pay_cost(game, seat, *id, c)
                    }
                    _ => true,
                })
            })
            .filter_map(|(i, a)| {
                a.mana().map(|m| match a {
                    // Heritage Druid: `getNetMana` while checking what's playable, its mana for each three it can tap
                    AbilityDef::ColoredManaAbility { color, amount, cost, no_tap: true, net, .. } => {
                        let counted = net.as_ref().map(|n| game.value(n, seat, *id, 0).max(0) as usize);
                        let times = counted
                            .or_else(|| {
                                cost.iter().find_map(|c| match c {
                                    CostDef::TapTargetCost { filter, amount } if *amount > 0 => Some(
                                        (mana_tap_targets(game, seat, *id, filter).len()
                                            + usize::from(game.filter_matches(filter, seat, Some(*id), *id)))
                                            / *amount,
                                    ),
                                    _ => None,
                                })
                            })
                            .unwrap_or(1);
                        (i, Mana::of(*color, *amount * times as u32), a.mana_kind())
                    }
                    AbilityDef::DynamicManaAbility { color, amount, .. } => {
                        let n = game.value(amount, seat, *id, 0).max(0) as u32;
                        (i, Mana::of(*color, n), a.mana_kind())
                    }
                    AbilityDef::ColorlessManaAbility { any_color_with: Some(counter), .. } => {
                        if o.counter(counter) > 0 {
                            (i, Mana { colored: [1; 5], ..Mana::default() }, ManaKind::AnyColor)
                        } else {
                            (i, Mana { colorless: 1, ..Mana::default() }, a.mana_kind())
                        }
                    }
                    AbilityDef::ColorlessManaAbility { two_if_exiled: true, .. } => {
                        let zcc = o.zcc;
                        let exiled = game
                            .exiled_for_source
                            .iter()
                            .any(|(s, z, c)| *s == *id && *z == zcc && game.obj(*c).zone == Zone::Exile);
                        (i, Mana { colorless: if exiled { 2 } else { 1 }, ..Mana::default() }, a.mana_kind())
                    }
                    AbilityDef::ConditionalColorManaAbility { color, otherwise, condition, .. } => {
                        let c = if game.condition_holds(Some(condition), seat, *id) { color } else { otherwise };
                        (i, Mana::of(*c, 1), a.mana_kind())
                    }
                    AbilityDef::NykthosShrineToNyxManaAbility { .. } => {
                        let mut m = Mana::default();
                        for c in Color::ALL {
                            m.colored[c.index()] = devotion(game, seat, c);
                        }
                        (i, m, a.mana_kind())
                    }
                    AbilityDef::AnyColorPermanentTypesManaAbility { filter, .. } => {
                        let mut m = Mana::default();
                        for c in colors_among(game, seat, *id, filter) {
                            m.colored[c.index()] = 1;
                        }
                        (i, m, a.mana_kind())
                    }
                    _ => (i, m, a.mana_kind()),
                })
            })
            .collect();
        if !abilities.is_empty() {
            out.push((*id, abilities));
        }
    }
    out
}

/// `AlternateManaPaymentAbility.getManaOptions`, as `PlayerImpl.findActivatedAbilityFromPlayable` adds it to the
/// mana a spell could be paid with: Delve, {1} per other card in your graveyard; Convoke, {1} or one of its colors
/// per untapped creature you control (mana creatures count twice, as in Java).
fn alternate_mana(game: &Game, seat: Seat, id: ObjId, def: &CardDef) -> Vec<Vec<Mana>> {
    let mut out = Vec::new();
    if def.has(&AbilityDef::DelveAbility) {
        let n = game.player(seat).graveyard.iter().filter(|c| **c != id).count();
        out.extend(std::iter::repeat_n(vec![Mana::default()], n));
    }
    if def.has(&AbilityDef::ImproviseAbility) {
        let n = game
            .battlefield
            .iter()
            .filter(|c| {
                let o = game.obj(**c);
                o.controller == seat && !o.tapped && game.has_type(**c, CardType::Artifact)
            })
            .count();
        out.extend(std::iter::repeat_n(vec![Mana::default()], n));
    }
    if def.has(&AbilityDef::ConvokeAbility) {
        for c in game.battlefield.iter() {
            let o = game.obj(*c);
            if o.controller == seat && !o.tapped && game.has_type(*c, CardType::Creature) {
                let mut m = vec![Mana::default()];
                m.extend(o.def.colors.iter().map(|col| Mana::of(*col, 1)));
                out.push(m);
            }
        }
    }
    out
}

/// Whether the pool and untapped sources (one mana each, any of its abilities) can pay a cost.
fn can_pay(game: &Game, seat: Seat, cost: &ManaCost) -> bool {
    can_pay_extra(game, seat, cost, Vec::new())
}

/// "whenever you tap a creature for mana, add an additional {G}" (Badgermole Cub): the colors tapping `id` for mana
/// adds besides its own
fn tap_for_mana_extra(game: &Game, seat: Seat, id: ObjId) -> Vec<usize> {
    let mut extra = Vec::new();
    for b in game.battlefield.iter() {
        let s = game.obj(*b);
        if s.controller != seat {
            continue;
        }
        for a in &s.def.abilities {
            if let AbilityDef::TapForManaAllTriggeredManaAbility { filter, color, .. } = a {
                if game.filter_matches(filter, seat, Some(*b), id) {
                    extra.push("WUBRG".find(color.as_str()).unwrap_or(4));
                }
            }
        }
    }
    extra
}

/// a player's devotion to a color
pub fn devotion(game: &Game, seat: Seat, c: Color) -> u32 {
    game.value(&Amount::Of(crate::card::Value::Devotion { c, c2: None }), seat, ObjId(0), 0).max(0) as u32
}

/// `can_pay`, with more sources (each a list of the mana it could make; none: generic only).
fn can_pay_extra(game: &Game, seat: Seat, cost: &ManaCost, extra: Vec<Vec<Mana>>) -> bool {
    // a phyrexian symbol can be paid with 2 life
    if game.player(seat).life >= 2 && cost.phyrexian.iter().any(|n| *n > 0) {
        let mut c = cost.clone();
        for k in 0..5 {
            c.colored[k] -= c.phyrexian[k];
        }
        c.phyrexian = [0; 5];
        return can_pay_extra(game, seat, &c, extra);
    }
    let (options, _) = mana_options(game, seat, extra.clone(), false);
    if options_pay(&options, cost) {
        return true;
    }
    // mana abilities with a mana cost (Castle Garenbrig's {2}{G}{G} for six {G}, two Giant's Boulders' {1} for
    // {R}{R}): the others pay their costs and what their mana doesn't (`addManaWithCost`, each source in turn)
    let costly: Vec<(ObjId, Mana, ManaCost)> = mana_sources(game, seat)
        .into_iter()
        .flat_map(|(id, a)| a.into_iter().map(move |(i, m)| (id, i, m)))
        .filter_map(|(id, i, m)| {
            let c = game.obj(id).def.abilities[i].mana_ability_mana_cost()?;
            Some((id, m, ManaCost::parse(c).ok()?))
        })
        .take(5)
        .collect();
    if !costly.is_empty() {
        let mut chosen = Vec::new();
        if pay_with_costly(game, seat, cost, &extra, &costly, 0, &mut chosen) {
            return true;
        }
    }
    // Nykthos's other way: two other sources pay its {2} (Sunken Citadel's land-only mana too), and it makes its
    // devotion to a color
    let (options, devotions) = mana_options(game, seat, extra, true);
    devotions.iter().any(|(c, n)| {
        (0..options.len()).any(|a| {
            (a + 1..options.len()).any(|b| {
                let mut left: Vec<Vec<Mana>> =
                    options.iter().enumerate().filter(|(k, _)| *k != a && *k != b).map(|(_, o)| o.clone()).collect();
                left.extend((0..*n).map(|_| vec![Mana::of(*c, 1)]));
                options_pay(&left, cost)
            })
        })
    })
}

/// `can_pay_extra` with some of the mana abilities that have a mana cost used, from `costly[from..]`: each chosen
/// one (one per source) taps for one kind of mana it makes, and its cost (any one of its symbols, as
/// `ManaOptions.addManaWithCost` takes it) is owed too. `chosen`: (source, kind, mana made, cost symbol).
fn pay_with_costly(
    game: &Game,
    seat: Seat,
    cost: &ManaCost,
    extra: &[Vec<Mana>],
    costly: &[(ObjId, Mana, ManaCost)],
    from: usize,
    chosen: &mut Vec<(ObjId, usize, u32, ManaCost)>,
) -> bool {
    let owed = |k: usize| {
        if k < 5 {
            cost.colored[k] > 0 || cost.hybrid.iter().any(|(a, b)| *a == k || *b == k)
        } else {
            cost.colorless > 0
        }
    };
    for (n, (id, m, own)) in costly.iter().enumerate().skip(from) {
        if chosen.iter().any(|(c, ..)| c == id) {
            continue;
        }
        let makes = |k: usize| if k < 5 { m.colored[k] } else { m.colorless };
        // each kind owed it makes, and one other (for generic, any does)
        let mut kinds: Vec<usize> = (0..6).filter(|k| makes(*k) > 0 && owed(*k)).collect();
        kinds.extend((0..6).find(|k| makes(*k) > 0 && !owed(*k)));
        let mut symbols: Vec<ManaCost> = Vec::new();
        if own.generic > 0 {
            symbols.push(ManaCost { generic: own.generic, ..ManaCost::default() });
        }
        for c in 0..5 {
            if own.colored[c] > 0 {
                let mut one = ManaCost::default();
                one.colored[c] = 1;
                symbols.push(one);
            }
        }
        for k in kinds {
            for sym in &symbols {
                chosen.push((*id, k, makes(k), sym.clone()));
                // the chosen sources tap for these abilities: the others pay
                let others: Vec<Vec<Mana>> = {
                    let ids: Vec<ObjId> = chosen.iter().map(|(c, ..)| *c).collect();
                    let outer = CHECKING_MANA_COST.with(|f| std::mem::replace(&mut *f.borrow_mut(), ids));
                    let o = mana_options(game, seat, extra.to_vec(), false).0;
                    CHECKING_MANA_COST.with(|f| *f.borrow_mut() = outer);
                    o
                };
                let mut options = others;
                let mut r = cost.clone();
                for (_, k, made, sym) in chosen.iter() {
                    let one =
                        if *k < 5 { Mana::of(Color::ALL[*k], 1) } else { Mana { colorless: 1, ..Mana::default() } };
                    options.extend((0..*made).map(|_| vec![one]));
                    for c in 0..5 {
                        r.colored[c] += sym.colored[c];
                    }
                    r.generic += sym.generic;
                }
                if options_pay(&options, &r) || pay_with_costly(game, seat, cost, extra, costly, n + 1, chosen) {
                    chosen.pop();
                    return true;
                }
                chosen.pop();
            }
        }
    }
    false
}

/// The mana sources `can_pay_extra` counts, one list of what each could make. `nykthos_paid`: Nykthos's devotion
/// ability left out, its devotion to each color returned instead (and land-only mana counted, it can pay Nykthos).
fn mana_options(
    game: &Game,
    seat: Seat,
    extra: Vec<Vec<Mana>>,
    nykthos_paid: bool,
) -> (Vec<Vec<Mana>>, Vec<(Color, u32)>) {
    let mut devotions = Vec::new();
    let mut options: Vec<Vec<Mana>> = Vec::new();
    let sources = mana_sources(game, seat);
    // `ManaOptions.addManaWithCost` pays Nykthos's {2} from the options before it, its own mana among them: with
    // nothing else to make mana, its devotion ability can't be paid
    let pool = game.player(seat).mana_pool;
    let more = sources.len() > 1
        || !extra.is_empty()
        || pool.colorless > 0
        || pool.colored.iter().any(|n| *n > 0)
        || !game.player(seat).conditional_pool.is_empty();
    for (id, a) in sources {
        // the mana its tapping triggers: one more source each
        for k in tap_for_mana_extra(game, seat, id) {
            options.push(vec![Mana::of(Color::ALL[k], 1)]);
        }
        // Nykthos: its {2} and itself spent, the rest of its devotion more sources of that color
        if let Some((_, m)) = a
            .iter()
            .find(|(i, _)| matches!(game.obj(id).def.abilities[*i], AbilityDef::NykthosShrineToNyxManaAbility { .. }))
        {
            if nykthos_paid {
                // (or, as ManaOptions has it, its {2} paid by two other sources and its devotion made)
                devotions.extend(Color::ALL.iter().map(|c| (*c, m.colored[c.index()])).filter(|(_, n)| *n > 0));
                continue;
            }
            if let Some(c) = Color::ALL.iter().copied().max_by_key(|c| m.colored[c.index()]).filter(|_| more) {
                for _ in 3..m.colored[c.index()] {
                    options.push(vec![Mana::of(c, 1)]);
                }
            }
        }
        let a: Vec<(usize, Mana)> = a
            .into_iter()
            .filter(|(i, _)| {
                let a = &game.obj(id).def.abilities[*i];
                // (one with a mana cost is counted by `can_pay_extra`, with what pays for it)
                !matches!(a, AbilityDef::NykthosShrineToNyxManaAbility { .. }) && a.mana_ability_mana_cost().is_none()
            })
            .collect();
        // `getManaAvailable`: an ability that doesn't tap the source is a source of its own (Relic of Legends'
        // "tap an untapped legendary creature"), the ones that tap it one source between them
        let (taps, own): (Vec<_>, Vec<_>) =
            a.into_iter().partition(|(i, _)| game.obj(id).def.abilities[*i].taps_source());
        let groups = std::iter::once(taps.into_iter().map(|(_, m)| m).collect::<Vec<Mana>>())
            .chain(own.into_iter().map(|(_, m)| vec![m]));
        for a in groups {
            if a.is_empty() {
                continue;
            }
            // an ability making several mana (a Lotus's three of one color, Sunken Citadel's two): a source for each
            let n = a
                .iter()
                .map(|m| m.colored.iter().copied().max().unwrap_or(0).max(m.colorless).max(1))
                .max()
                .unwrap_or(1);
            for _ in 0..n {
                options.push(a.clone());
            }
        }
    }
    options.extend(extra);
    // conditional mana left in the pool (counted for any spell, as Java's playable check does)
    if game.paying_for.get() == Some(ANY_SPELL) {
        // each mana its own source (Nykthos's or Sunken Citadel's two are one entry)
        for (m, _, _) in &game.player(seat).conditional_pool {
            for c in Color::ALL {
                for _ in 0..m.colored[c.index()] {
                    options.push(vec![Mana::of(c, 1)]);
                }
            }
            for _ in 0..m.colorless {
                options.push(vec![Mana { colorless: 1, ..Mana::default() }]);
            }
        }
    }
    let pool = game.player(seat).mana_pool;
    for c in Color::ALL {
        for _ in 0..pool.colored[c.index()] {
            options.push(vec![Mana::of(c, 1)]);
        }
    }
    for _ in 0..pool.colorless {
        options.push(vec![Mana { colorless: 1, ..Mana::default() }]);
    }
    (options, devotions)
}

/// Whether the sources pay the cost: each symbol from a different source, generic from what's left.
fn options_pay(options: &[Vec<Mana>], cost: &ManaCost) -> bool {
    // the costs' symbols, colored and colorless first; each needs a different source
    let mut needs: Vec<usize> = Vec::new();
    for c in Color::ALL {
        needs.extend(std::iter::repeat_n(c.index(), cost.colored[c.index()] as usize));
    }
    needs.extend(std::iter::repeat_n(5, cost.colorless as usize));
    // a hybrid symbol: 6 + its two colors
    needs.extend(cost.hybrid.iter().map(|(a, b)| 6 + a * 5 + b));
    let mut used = vec![false; options.len()];
    fn fits(m: &Mana, k: usize) -> bool {
        if k < 5 {
            m.colored[k] > 0
        } else if k == 5 {
            m.colorless > 0
        } else {
            m.colored[(k - 6) / 5] > 0 || m.colored[(k - 6) % 5] > 0
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
    if !assign(&needs, options, &mut used) {
        return false;
    }
    used.iter().filter(|u| !**u).count() as u32 >= cost.generic
}

/// `ManaUtil.tryToAutoPay`: with only basic and any-color abilities, the one ability that pays the one mana symbol
/// still owed (a color, or a hybrid symbol one of its colors pays); with no symbol owed and generic left, the first
/// (the first that only taps, when nothing colored is owed at all); otherwise all of them (the player picks).
fn auto_pay_abilities(abilities: &[(usize, Mana, ManaKind)], unpaid: &ManaCost) -> Vec<(usize, Mana, ManaKind)> {
    if abilities.iter().any(|a| a.2 == ManaKind::Other) {
        return abilities.to_vec();
    }
    let hybrid: Vec<(usize, usize)> = unpaid.hybrid.iter().map(|(a, b)| ((*a).min(*b), (*a).max(*b))).collect();
    let single = |k: usize| k < 5 && unpaid.colored[k] > 0;
    if (0..5).all(|k| !single(k)) && hybrid.is_empty() {
        // `getManaAbilitiesUsingMana`
        if unpaid.generic > 0 {
            let no_colors = unpaid.colorless == 0;
            let plain = abilities.iter().find(|a| a.2 == ManaKind::Basic).filter(|_| no_colors);
            return vec![*plain.unwrap_or(&abilities[0])];
        }
        return abilities.to_vec();
    }
    // `getManaAbilitiesUsingManaSymbols`: a basic ability's color (WUBRG), and the hybrid symbols it looks for, in
    // the order `checkWhiteMana` and the rest do
    const PARTNERS: [[usize; 5]; 5] =
        [[1, 2, 4, 3, 5], [2, 3, 0, 4, 5], [3, 4, 1, 0, 5], [2, 4, 0, 1, 5], [0, 1, 2, 3, 5]];
    let color = |(_, m, k): &(usize, Mana, ManaKind)| {
        (*k == ManaKind::Basic)
            .then(|| (0..5).find(|c| m.colored[*c] > 0 && m.colored.iter().sum::<u32>() == m.colored[*c]))
            .flatten()
    };
    let mut count = std::collections::BTreeSet::new();
    let mut chosen = None;
    let pays_single = |a: &(usize, Mana, ManaKind), count: &mut std::collections::BTreeSet<(usize, usize)>| {
        let c = color(a)?;
        single(c).then(|| {
            count.insert((c, c));
        })
    };
    for a in abilities {
        if pays_single(a, &mut count).is_some() {
            chosen = Some(*a);
        }
        if let Some(c) = color(a) {
            if let Some(h) = PARTNERS[c].iter().map(|p| (c.min(*p), c.max(*p))).find(|h| hybrid.contains(h)) {
                chosen = Some(*a);
                count.insert(h);
            }
        }
    }
    match count.len() {
        0 if unpaid.generic > 0 => vec![abilities[0]],
        0 => abilities.to_vec(),
        1 => chosen.into_iter().collect(),
        _ => {
            // a hybrid symbol drops out when exactly one of its colors is owed on its own
            let left =
                count.iter().filter(|(a, b)| a == b || count.contains(&(*a, *a)) == count.contains(&(*b, *b))).count();
            if left == 1 {
                for a in abilities {
                    if pays_single(a, &mut count).is_some() {
                        chosen = Some(*a);
                    }
                }
                chosen.into_iter().collect()
            } else {
                abilities.to_vec()
            }
        }
    }
}

/// The colors among the permanents of `filter` `seat` controls (`AnyColorPermanentTypesManaEffect.getManaTypes`)
fn colors_among(game: &Game, seat: Seat, source: ObjId, filter: &crate::card::Filter) -> Vec<Color> {
    let mut out = Vec::new();
    for o in matching(game, filter, seat, source) {
        if game.obj(o).controller != seat {
            continue;
        }
        for c in game.colors(o) {
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

/// One of `allowed` (`ManaType.getChoiceOfManaTypes`, black red blue green white): the only one; while paying, the
/// one owed (`ManaUtil.tryToAutoSelectAManaColor`), the first with no color owed; else the player picks it. None:
/// stopped.
fn color_among(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    unpaid: &ManaCost,
    allowed: &[Color],
) -> Option<Color> {
    let order = [Color::B, Color::R, Color::U, Color::G, Color::W];
    let choices: Vec<Color> = order.iter().copied().filter(|c| allowed.contains(c)).collect();
    if choices.len() == 1 {
        return Some(choices[0]);
    }
    let owed: Vec<Color> = [Color::W, Color::R, Color::G, Color::U, Color::B]
        .into_iter()
        .filter(|c| unpaid.symbols_of(*c) > 0 && choices.contains(c))
        .collect();
    if owed.len() == 1 {
        return Some(owed[0]);
    }
    if owed.is_empty() && Color::ALL.iter().all(|c| unpaid.symbols_of(*c) == 0) {
        return choices.first().copied();
    }
    let name = |c: &Color| ["White", "Blue", "Black", "Red", "Green"][c.index()].to_string();
    let q = Query::Choice { message: "Pick a mana color".into(), choices: choices.iter().map(name).collect() };
    loop {
        match ask(game, d, seat, &q) {
            Answer::Text(t) => {
                if let Some(c) = choices.iter().find(|c| name(c) == t) {
                    return Some(*c);
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

/// `AddConditionalManaOfAnyColorEffect.produceMana` for more than one mana in any combination of colors: the player
/// divides it among W, U, B, R and G (`getMultiAmount`, all on white by default). None: not such an ability; Some(None):
/// stopped.
fn mana_combination(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, i: usize) -> Option<Option<Mana>> {
    let AbilityDef::ConditionalManaAbility { amount, combination: true, .. } = game.obj(id).def.abilities[i] else {
        return None;
    };
    if amount < 2 {
        return None;
    }
    let n = amount as i32;
    let mut defaults = vec![0; 5];
    defaults[0] = n;
    let Some(values) = multi_amount(game, d, seat, defaults, vec![(0, n); 5], (n, n), (None, Vec::new())) else {
        return Some(None);
    };
    let mut m = Mana::default();
    for (c, v) in values.iter().enumerate() {
        m.colored[c] = *v as u32;
    }
    Some(Some(m))
}

/// `ManaUtil.tryToAutoSelectAManaColor`: the one color still owed; with none owed, white; else the player chooses
/// (CHOOSE_CHOICE). None: stopped.
fn any_color(game: &mut Game, d: &mut dyn Decider, seat: Seat, unpaid: &ManaCost, n: u32) -> Option<Color> {
    let owed: Vec<Color> = Color::ALL.iter().copied().filter(|c| unpaid.symbols_of(*c) > 0).collect();
    match owed.len() {
        0 => {
            game.any_color_made = Some(Color::W);
            return Some(Color::W);
        }
        1 => {
            game.any_color_made = Some(owed[0]);
            return Some(owed[0]);
        }
        _ => {}
    }
    let names = ["White", "Blue", "Black", "Red", "Green"];
    let choices: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    let q = Query::Choice { message: format!("Select a color of mana to add {n} of it"), choices };
    loop {
        match ask(game, d, seat, &q) {
            Answer::Text(t) => {
                if let Some(c) =
                    names.iter().position(|n| *n == t).map(|i| [Color::W, Color::U, Color::B, Color::R, Color::G][i])
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
        // (conditional ones too: their mana waits in the pool for a spell it can pay for)
        game.paying_for.set(Some(ANY_SPELL));
        let mana: Vec<(usize, Mana, ManaKind)> =
            mana_sources_with_kinds(game, seat).into_iter().find(|(s, _)| *s == id).map(|(_, a)| a).unwrap_or_default();
        game.paying_for.set(None);
        let mut abilities = activatable.clone();
        abilities.extend(mana.iter().map(|(i, _, _)| *i));
        abilities.sort();
        // `HumanPlayer.activateAbility`: one ability with mana in its cost, or a mana ability, activates directly;
        // otherwise the picker
        let direct = abilities.len() == 1
            && (mana.iter().any(|(i, _, _)| *i == abilities[0])
                || matches!(game.obj(id).def.abilities[abilities[0]], AbilityDef::RoomUnlockAbility { .. })
                || {
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
            let before = game.player(seat).mana_pool;
            let conditional = matches!(game.obj(id).def.abilities[i], AbilityDef::ConditionalManaAbility { .. });
            // mana of any color: the color is asked, nothing being paid (`HumanPlayer.choose` autoselects only
            // while paying)
            let owed = ManaCost { colored: [1; 5], ..ManaCost::default() };
            if activate_mana(game, d, seat, id, i, m, k, &owed) {
                if conditional {
                    let p = game.player_mut(seat);
                    let mut made = Mana::default();
                    for c in 0..5 {
                        made.colored[c] = p.mana_pool.colored[c] - before.colored[c];
                    }
                    p.mana_pool = before;
                    p.conditional_pool.push((made, id, i));
                }
                for s in 0..game.seats() {
                    game.player_mut(s).passed = false;
                }
            }
            return;
        }
        let before = game.clone();
        // `RoomUnlockAbility`, a special action: its cost paid, the door unlocks (no stack)
        if let AbilityDef::RoomUnlockAbility { left, cost } = game.obj(id).def.abilities[index].clone() {
            let paid = ManaCost::parse(&cost).is_ok_and(|c| {
                game.paying_for.set(Some(id));
                let paid = pay_mana(game, d, seat, &c);
                game.paying_for.set(None);
                paid
            });
            if !paid {
                *game = before;
                return;
            }
            unlock_door(game, id, if left { 0 } else { 1 });
            for s in 0..game.seats() {
                game.player_mut(s).passed = false;
            }
            return;
        }
        if !activate(game, d, seat, id, index) {
            *game = before;
        }
        return;
    }
    // a card in hand with an ability (cycling, channel): the picker, with its spell or land play (a0) and the abilities, if there is a choice
    // (or one you may play from exile, with its spectacle cost)
    let zone = game.obj(id).zone;
    // a card exiled with Valgavoth, cast for life
    if zone == Zone::Exile && !game.obj(id).is(CardType::Land) && valgavoth_play(game, seat, id) {
        let before = game.clone();
        let full = game.obj(id).def.clone();
        let o = game.obj_mut(id);
        o.def = Arc::new(valgavoth_def(&full));
        o.adventurer = Some(full);
        if !cast(game, d, seat, id) {
            *game = before;
        }
        return;
    }
    // a plotted card (cast free, as a sorcery) or a foretold one (for its foretell cost), on a later turn
    if zone == Zone::Exile && (game.obj(id).plotted_turn.is_some() || game.obj(id).foretold_turn.is_some()) {
        let Some((def, free)) = exile_cast(game, seat, id) else { return };
        // `lookAtFaceDownCard`: a foretold card is face down, so its owner is asked whether to only look at it
        if game.obj(id).foretold_turn.is_some() {
            let q = Query::YesNo { message: format!("Look at {}", game.obj(id).def.name) };
            let look = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return,
                    _ => {}
                }
            };
            if look {
                return;
            }
        }
        if free {
            // `PlotSpellAbility`, added after the card's own abilities: free, so the picker isn't suppressed
            // (`HumanPlayer.suppressAbilityPicker`) and shows its one choice
            let k = game.obj(id).def.abilities.len();
            match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: vec![k] }) {
                Answer::Ability(o, i) if o == id && i == k => {}
                _ => return,
            }
            cast_free(game, d, seat, id);
            return;
        }
        let before = game.clone();
        let full = game.obj(id).def.clone();
        let o = game.obj_mut(id);
        o.adventurer = Some(full);
        o.def = Arc::new(def);
        if !cast(game, d, seat, id) {
            *game = before;
        }
        return;
    }
    if (zone == Zone::Hand || zone == Zone::Exile) && game.obj(id).def.adventure.is_none() {
        let mut options = if zone == Zone::Hand { activatable(game, seat, id) } else { Vec::new() };
        // foretell, a special action: during your turn, with {2} to pay
        let foretell = foretell(game, seat, id);
        if let Some(k) = foretell {
            options.push(k);
        }
        let plot = if zone == Zone::Hand { plot(game, seat, id) } else { None };
        if let Some((k, _)) = &plot {
            options.push(*k);
        }
        let suspend = if zone == Zone::Hand { suspend(game, seat, id) } else { None };
        if let Some((k, _, _)) = &suspend {
            options.push(*k);
        }
        let spectacle = spectacle(game, seat, id);
        if let Some((k, _)) = &spectacle {
            options.push(*k);
        }
        let warp = warp(game, seat, id);
        if let Some((k, _)) = &warp {
            options.push(*k);
        }
        let cleave = cleave(game, seat, id);
        if let Some((k, _)) = &cleave {
            options.push(*k);
        }
        let overload = overload(game, seat, id);
        if let Some((k, _)) = &overload {
            options.push(*k);
        }
        let blitz = if zone == Zone::Hand { blitz(game, seat, id) } else { None };
        if let Some((k, _)) = &blitz {
            options.push(*k);
        }
        // a modal double-faced card with a spell front and a land back (Malakir Rebirth): its back's play ability,
        // after the front's abilities
        let land_back = (zone == Zone::Hand && !game.obj(id).def.is(CardType::Land))
            .then(|| game.obj(id).def.back.clone())
            .flatten()
            .filter(|b| b.is(CardType::Land) && can_play_land(game, seat))
            .and_then(|b| b.abilities.iter().position(|a| *a == AbilityDef::PlayLandAbility))
            .map(|k| game.obj(id).def.abilities.len() + k);
        if let Some(k) = land_back {
            options.push(k);
        }
        // its front cast, or its back played: Java shows the picker for either, even with one choice
        let mdfc = zone == Zone::Hand && !game.obj(id).def.is(CardType::Land) && game.obj(id).def.back.is_some();
        if !options.is_empty() || mdfc {
            let def = game.obj(id).def.clone();
            if if def.is(CardType::Land) { can_play_land(game, seat) } else { castable(game, seat, id, &def) } {
                options.insert(0, 0);
            }
            // (a modal double-faced card's faces always show the picker, one choice or two)
            if options.is_empty() {
                return;
            }
            let index = if options.len() == 1 && !mdfc {
                options[0]
            } else {
                match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: options.clone() }) {
                    Answer::Ability(o, i) if o == id && options.contains(&i) => i,
                    _ => return,
                }
            };
            // cast for its spectacle or warp cost (the whole card comes back as it leaves the stack)
            let alternative = spectacle
                .filter(|(k, _)| *k == index)
                .or(cleave.filter(|(k, _)| *k == index))
                .or(overload.filter(|(k, _)| *k == index))
                .map(|(_, def)| (def, false));
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
            if let Some((_, def)) = blitz.filter(|(k, _)| *k == index) {
                cast_blitzed(game, d, seat, id, def);
                return;
            }
            if let Some((_, time, cost)) = suspend.filter(|(k, _, _)| *k == index) {
                // `SuspendExileEffect`: pay the suspend cost, then exile it with its time counters
                let before = game.clone();
                if !pay_mana(game, d, seat, &cost) {
                    *game = before;
                    return;
                }
                game.move_to(id, Zone::Exile);
                game.obj_mut(id).add_counters("time", time);
                game.obj_mut(id).suspended = true;
                if log::kept(game) {
                    let line = format!("{} suspends ({}) {}", log::player(seat), time, log::object(game, id));
                    log::inform(game, line);
                }
                for s in 0..game.seats() {
                    game.player_mut(s).passed = false;
                }
                return;
            }
            if let Some((_, cost)) = plot.filter(|(k, _)| *k == index) {
                // `PlotSourceExileEffect`: pay the plot cost, then exile it plotted; it can't be cast this turn
                let before = game.clone();
                if !pay_mana(game, d, seat, &cost) {
                    *game = before;
                    return;
                }
                game.move_to(id, Zone::Exile);
                game.obj_mut(id).plotted_turn = Some(game.turn);
                for s in 0..game.seats() {
                    game.player_mut(s).passed = false;
                }
                return;
            }
            if foretell == Some(index) {
                // `ForetellExileEffect`: pay {2}, then exile it face down; it can't be cast this turn
                let before = game.clone();
                if !pay_mana(game, d, seat, &ManaCost { generic: 2, ..ManaCost::default() }) {
                    *game = before;
                    return;
                }
                game.move_to(id, Zone::Exile);
                game.obj_mut(id).foretold_turn = Some(game.turn);
                for s in 0..game.seats() {
                    game.player_mut(s).passed = false;
                }
                return;
            }
            if land_back == Some(index) {
                // played as its back: the land below
                let full = game.obj(id).def.clone();
                let back = full.back.clone().expect("a land back");
                let o = game.obj_mut(id);
                o.front = Some(full);
                o.def = back;
            } else if index != 0 {
                let before = game.clone();
                if !activate(game, d, seat, id, index) {
                    *game = before;
                }
                return;
            }
        }
    }
    // a land with an adventure-like spell (Midgar, City of Mako): with a land to play, the picker offers the spell
    // (a0) whatever it costs and the land (a1), as `playCard` does for a card with two spell abilities; back from its
    // adventure, the same picker, though only the land can be played from exile
    let on_adventure = game.obj(id).zone == Zone::Exile && game.obj(id).on_adventure;
    if game.obj(id).is(CardType::Land) && (game.obj(id).zone == Zone::Hand || on_adventure) {
        if let Some(adventure) = game.obj(id).def.adventure.clone() {
            let options = if can_play_land(game, seat) {
                vec![0, 1]
            } else if !on_adventure && castable(game, seat, id, &adventure) {
                vec![0]
            } else {
                return;
            };
            let k = if options.len() == 1 {
                options[0]
            } else {
                match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: options.clone() }) {
                    Answer::Ability(o, k) if o == id && options.contains(&k) => k,
                    _ => return,
                }
            };
            if k == 0 && on_adventure {
                return;
            }
            if k == 0 {
                let before = game.clone();
                let full = game.obj(id).def.clone();
                let o = game.obj_mut(id);
                o.adventurer = Some(full);
                o.def = adventure;
                if !cast(game, d, seat, id) {
                    *game = before;
                }
                return;
            }
        }
    }
    // a land in a graveyard with an ability of its own there (Drownyard Temple): that ability, below
    let graveyard_ability = game.obj(id).zone == Zone::Graveyard && !activatable(game, seat, id).is_empty();
    if game.obj(id).is(CardType::Land) && !graveyard_ability {
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
        note(game, EventKind::Land, Some(seat), Some(id), None, 0);
        if log::kept(game) {
            // `PlayerImpl.playLand`
            let line = match game
                .obj(id)
                .front
                .clone()
                .or_else(|| game.obj(id).def.back.as_ref().map(|_| game.obj(id).def.clone()))
            {
                // a modal double-faced card's face (`GameLog.replaceNameByColoredName`)
                Some(main) => {
                    let r = crate::replay::reference(game, id);
                    let color = crate::prompt::log_color(&game.colors(id));
                    format!(
                        "{} plays <font color='{}' object_id='{}' alternative_name='{}'>{}</font> [{}] as MDF side of <font color='{}' object_id='{}f0'>{}</font> [{}f0]",
                        log::player(seat), color, r, main.name.replace(' ', "+"), game.obj(id).name(), r,
                        crate::prompt::log_color(&main.colors), r, main.name, r
                    )
                }
                None => format!("{} plays {}", log::player(seat), log::object(game, id)),
            };
            log::inform(game, line);
        }
        return;
    }
    let before = game.clone();
    // an aftermath card: its left half cast from hand, its right half from the graveyard (then exiled), the half
    // always asked (`a0`, `a1`)
    if game.obj(id).def.aftermath && matches!(game.obj(id).zone, Zone::Hand | Zone::Graveyard) {
        let full = game.obj(id).def.clone();
        let k = usize::from(game.obj(id).zone == Zone::Graveyard);
        let Some(half) = full.room.get(k).cloned() else { return };
        if !castable(game, seat, id, &half) {
            return;
        }
        match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: vec![k] }) {
            Answer::Ability(o, i) if o == id && i == k => {}
            _ => return,
        }
        let o = game.obj_mut(id);
        o.adventurer = Some(full);
        o.def = half;
        if !cast(game, d, seat, id) {
            *game = before;
            return;
        }
        if k == 1 {
            game.obj_mut(id).flashback = true;
        }
        return;
    }
    // flashback: cast with its flashback cost (the whole card comes back as it leaves the stack)
    if game.obj(id).zone == Zone::Graveyard {
        // an ability that works from the graveyard (Dread Wanderer): with mana in its cost, activated directly
        let abilities = activatable(game, seat, id);
        if !abilities.is_empty() {
            // the picker is suppressed only for one ability with mana in its cost (Cauldron Familiar's is shown)
            let index = if abilities.len() == 1
                && game.obj(id).def.abilities[abilities[0]]
                    .activated()
                    .is_some_and(|(_, _, _, costs, _)| ability_mana_cost(costs).mana_value() > 0)
            {
                abilities[0]
            } else {
                match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: abilities.clone() }) {
                    Answer::Ability(o, i) if o == id && abilities.contains(&i) => i,
                    _ => return,
                }
            };
            if !activate(game, d, seat, id, index) {
                *game = before;
            }
            return;
        }
        if let Some((_, def)) = blitz(game, seat, id) {
            cast_blitzed(game, d, seat, id, def);
            return;
        }
        // a card you may cast from your graveyard (Case of the Uneaten Feast): cast as from hand
        if game.flashback_def(id).is_none()
            && game.obj(id).may_play.is_some_and(|(p, t)| p == seat && game.turn <= t)
            && castable(game, seat, id, &{
                let def = (*game.obj(id).def).clone();
                if game.obj(id).play_free {
                    CardDef { cost: ManaCost::default(), ..def }
                } else {
                    def
                }
            })
        {
            // `suppressAbilityPicker`: a {0} spell is picked (a0)
            if game.obj(id).def.cost.mana_value() == 0 {
                match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: vec![0] }) {
                    Answer::Ability(o, 0) if o == id => {}
                    _ => return,
                }
            }
            let cast = if game.obj(id).play_free { cast_free } else { cast };
            if !cast(game, d, seat, id) {
                *game = before;
            }
            return;
        }
        if let Some(f) = game.flashback_def(id) {
            // `HumanPlayer.activateAbility`: the picker is suppressed only for an ability with mana in its cost
            // (Lava Dart's flashback, a sacrifice, is picked)
            if f.cost.mana_value() == 0 {
                if let Some(k) =
                    game.obj(id).def.abilities.iter().position(|a| matches!(a, AbilityDef::FlashbackAbility { .. }))
                {
                    match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: vec![k] }) {
                        Answer::Ability(o, i) if o == id && i == k => {}
                        _ => return,
                    }
                }
            }
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
        // escape: the card cast with its escape cost and exile cost
        let Some(f) = game.obj(id).def.escape() else { return };
        let full = game.obj(id).def.clone();
        let o = game.obj_mut(id);
        o.adventurer = Some(full);
        o.def = Arc::new(f);
        if !cast(game, d, seat, id) {
            *game = before;
            return;
        }
        game.obj_mut(id).escaped = true;
        return;
    }
    // a Room: the half to cast, always asked (h0, h1), even with one that can be
    if !game.obj(id).def.room.is_empty() {
        let full = game.obj(id).def.clone();
        let options: Vec<usize> = (0..full.room.len()).filter(|k| castable(game, seat, id, &full.room[*k])).collect();
        if options.is_empty() {
            return;
        }
        let k = match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: options.clone() }) {
            Answer::Ability(o, k) if o == id && options.contains(&k) => k,
            _ => return,
        };
        let o = game.obj_mut(id);
        o.adventurer = Some(full.clone());
        o.def = full.room[k].clone();
        o.room_cast = Some(k);
        if !cast(game, d, seat, id) {
            *game = before;
        }
        return;
    }
    // an adventurer card: the picker, with the adventure (a0) and the creature (a1) that can be cast
    if let Some(adventure) = game.obj(id).def.adventure.clone() {
        let o = game.obj(id);
        let mut options = Vec::new();
        // from hand, or from exile with permission to play it (not the creature of a card on an adventure)
        if (o.zone == Zone::Hand || (o.zone == Zone::Exile && !o.on_adventure)) && castable(game, seat, id, &adventure)
        {
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
    // `suppressAbilityPicker`: one spell ability is cast directly only with mana to pay (X counts 0), so a {0} or
    // {X} spell is picked (a0)
    if game.obj(id).def.cost.mana_value() == 0 {
        match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: vec![0] }) {
            Answer::Ability(o, 0) if o == id => {}
            _ => return,
        }
    }
    // played from exile without paying its mana cost (Oracle's Vault)
    let free = game.obj(id).zone == Zone::Exile && game.obj(id).play_free;
    if !(if free { cast_free(game, d, seat, id) } else { cast(game, d, seat, id) }) {
        // `PlayerImpl.cast` restores the game as it was before the cast
        *game = before;
    }
}

/// Casts a spell from hand: onto the stack, targets, then mana. False: cancelled.
fn cast(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId) -> bool {
    cast_with(game, d, seat, id, false)
}

/// `cast`, `free` without paying its mana cost (X is 0; additional costs are still paid). The game is as it was
/// before when it's cancelled.
fn cast_free(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId) -> bool {
    let before = game.clone();
    if cast_with(game, d, seat, id, true) {
        return true;
    }
    *game = before;
    false
}

/// A cast an effect allows (paying its costs): the game is as it was before when it's cancelled
fn cast_allowed(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId) -> bool {
    let before = game.clone();
    // `chooseAbilityForCast`: an adventurer card, the adventure (a0) or the creature (a1). Both callers cast with
    // `playCard`, which allows every spell ability of the card (`PlayFromNotOwnHandZone`), so
    // `getCastableSpellAbilities` offers both whatever they cost: one the player can't pay is cancelled while paying
    if game.obj(id).def.adventure.is_some() {
        let options = vec![0, 1];
        let pick = match ask(game, d, seat, &Query::ChooseAbility { source: id, abilities: options.clone() }) {
            Answer::Ability(o, k) if o == id && options.contains(&k) => k,
            Answer::Stop => return false,
            _ => 1,
        };
        let adventure = game.obj(id).def.adventure.clone();
        if let Some(adventure) = adventure.filter(|_| pick == 0) {
            let full = game.obj(id).def.clone();
            let o = game.obj_mut(id);
            o.adventurer = Some(full);
            o.def = adventure;
        }
    }
    if cast_with(game, d, seat, id, false) {
        return true;
    }
    *game = before;
    false
}

fn cast_with(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, free: bool) -> bool {
    let outer = game.unpaid.replace(game.stack.len());
    let cast = cast_paying(game, d, seat, id, free);
    game.unpaid = outer;
    game.mana_paid = None;
    // `processAction` once it's cast
    game.short_lki.clear();
    cast
}

fn cast_paying(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, free: bool) -> bool {
    let from = game.obj(id).zone;
    game.obj_mut(id).cast_from = Some(from);
    // a prepared copy: casting it unprepares its permanent
    if let Some((p, zcc)) = game.obj(id).prepared_for {
        if game.obj(p).zcc == zcc && game.obj(p).prepared_copy == Some(id) {
            game.obj_mut(p).prepared_copy = None;
        }
    }
    game.remove_from_zone(id);
    {
        let o = game.obj_mut(id);
        o.zone = Zone::Stack;
        o.zcc += 1;
        o.on_adventure = false;
        o.mana_spent = 0;
    }
    Arc::make_mut(&mut game.stack).push(StackItem::spell(id, seat));
    let def = game.obj(id).def.clone();
    let mut cost = if free { ManaCost::default() } else { def.cost.clone() };
    let mut alternative_costs: Vec<CostDef> = Vec::new();
    // splice (601.2b, `ContinuousEffects.applySpliceEffects`): before the other costs
    let Some(splice_cost) = choose_splices(game, d, seat, id, &def) else { return false };
    cost = cost.plus(&splice_cost);
    // evoke: `AbilityImpl.activate` offers the alternative costs first (not when cast without paying, or for an
    // alternative cost already: flashback, escape…)
    if !free && def.printed_cost.is_none() {
        if let Some((evoke, keyword, evoke_costs)) = def.abilities.iter().find_map(|a| match a {
            // (`AlternativeCostSourceAbility.askToActivateAlternativeCosts`: offered only if it can be paid)
            AbilityDef::EvokeAbility { cost, keyword, costs, .. }
                if costs.iter().all(|c| can_pay_cost(game, seat, id, c)) =>
            {
                Some((
                    cost.clone(),
                    if keyword.is_empty() { "Evoke".to_string() } else { keyword.clone() },
                    costs.clone(),
                ))
            }
            _ => None,
        }) {
            let choices = vec![
                format!("Cast with {keyword} alternative cost: {}", evoke),
                format!("Cast with no alternative cost: {}", def.cost.text),
            ];
            let q = Query::Choice { message: "You may choose an alternative cost".into(), choices: choices.clone() };
            let pick = loop {
                match ask(game, d, seat, &q) {
                    Answer::Text(t) if t == "1" || t.starts_with(&format!("Cast with {keyword}")) => break true,
                    Answer::Text(_) => break false,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pick {
                cost = if evoke.is_empty() {
                    ManaCost::default()
                } else {
                    let Ok(c) = ManaCost::parse(&evoke) else { return false };
                    c
                };
                alternative_costs = evoke_costs;
                if keyword == "Dash" {
                    game.obj_mut(id).dashed = true;
                } else {
                    game.obj_mut(id).evoked = true;
                }
            }
        }
    }
    // `AlternativeCostSourceAbility`: offered the same way, its costs paid with the others
    if !free && def.printed_cost.is_none() && !game.obj(id).evoked && !game.obj(id).dashed {
        if let Some((costs, rule)) = alternative_cost(game, seat, id, &def) {
            // the costs' text (Force of Negation's "exile a blue card from your hand")
            let text =
                rule.split(", you may ").nth(1).and_then(|r| r.split(" rather than").next()).unwrap_or("").to_string();
            let choices = vec![
                format!("Cast with alternative cost: {}", text),
                format!("Cast with no alternative cost: {}", def.cost.text),
            ];
            let q = Query::Choice { message: "You may choose an alternative cost".into(), choices: choices.clone() };
            let pick = loop {
                match ask(game, d, seat, &q) {
                    Answer::Text(t) if t == "1" || t.starts_with("Cast with alternative cost") => break true,
                    Answer::Text(_) => break false,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pick {
                cost = ManaCost::default();
                alternative_costs = costs;
            }
        }
    }
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
    // replicate: `ReplicateAbility.addOptionalAdditionalCosts` asks again after each payment
    for a in &def.abilities {
        if let AbilityDef::ReplicateAbility { cost: more, .. } = a {
            let Ok(k) = ManaCost::parse(more) else { return false };
            let mut times = 0;
            loop {
                let n = times + 1;
                let q = Query::YesNo {
                    message: format!(
                        "Pay {} Replicate {} ?",
                        if n == 1 { "1 time".to_string() } else { format!("{n} times") },
                        more
                    ),
                };
                let pay = loop {
                    match ask(game, d, seat, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return false,
                        _ => {}
                    }
                };
                if !pay {
                    break;
                }
                times += 1;
                cost = cost.plus(&k);
            }
            if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                top.replicated = times;
            }
        }
    }
    // teamwork: `TeamworkAbility.addOptionalAdditionalCosts`, asked only when the creatures can be tapped
    let mut teamwork = None;
    for a in &def.abilities {
        if let AbilityDef::TeamworkAbility { n, .. } = a {
            let c = CostDef::CrewCost { value: *n };
            if !can_pay_cost(game, seat, id, &c) {
                continue;
            }
            let q = Query::YesNo { message: format!("Teamwork {}?", n) };
            let pay = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pay {
                teamwork = Some(c);
                if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                    top.kicked = true;
                }
            }
        }
    }
    // blight: `BlightAbility.addOptionalAdditionalCosts`, asked only with a creature to put the counters on
    let mut blight = None;
    for a in &def.abilities {
        if let AbilityDef::BlightAbility { amount, .. } = a {
            if controlled_creatures(game, seat).is_empty() {
                continue;
            }
            let q = Query::YesNo { message: format!("Blight {}?", amount) };
            let pay = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pay {
                blight = Some(*amount);
                if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                    top.kicked = true;
                }
            }
        }
    }
    // casualty: `CasualtyAbility.addOptionalAdditionalCosts`, asked only when the sacrifice can be paid; "when you do,
    // copy this spell" waits from then on
    let mut casualty = None;
    for a in &def.abilities {
        if let AbilityDef::CasualtyAbility { n, .. } = a {
            let filter = crate::card::Filter(vec![
                Pred::Type { t: CardType::Creature },
                Pred::Power { cmp: Cmp::OrGreater, n: *n },
            ]);
            let c = CostDef::SacrificeTargetCost { filter, amount: 1 };
            if !can_pay_cost(game, seat, id, &c) {
                continue;
            }
            let q = Query::YesNo { message: format!("Sacrifice a creature with power {} or greater?", n) };
            let pay = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pay {
                casualty = Some(c);
                let delayed = Delayed {
                    rule: "When you do, copy this spell.".into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: vec![EffectDef::CopySourceSpellEffect],
                    source_zcc: game.obj(id).zcc,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, seat) });
            }
        }
    }
    // bargain: `BargainAbility.addOptionalAdditionalCosts`, asked only when the sacrifice can be paid
    let mut bargain = None;
    for a in &def.abilities {
        if let AbilityDef::BargainAbility { .. } = a {
            let c = bargain_cost();
            if !can_pay_cost(game, seat, id, &c) {
                continue;
            }
            let q =
                Query::YesNo { message: "Bargain? (To Bargain, sacrifice an artifact, enchantment, or token)".into() };
            let pay = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pay {
                bargain = Some(c);
                if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                    top.kicked = true;
                }
            }
        }
    }
    // gift: `GiftAbility.addOptionalAdditionalCosts`
    for a in &def.abilities {
        if let AbilityDef::GiftAbility { name, .. } = a {
            let q = Query::YesNo { message: format!("Promise an opponent {}?", name) };
            let promise = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if promise {
                if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                    top.gift_promised = true;
                }
            }
        }
    }
    // offspring: `OffspringAbility.addOptionalAdditionalCosts`
    for a in &def.abilities {
        if let AbilityDef::OffspringAbility { cost: more, .. } = a {
            let q = Query::YesNo { message: format!("Pay {} for offspring?", more) };
            let pay = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if pay {
                let Ok(k) = ManaCost::parse(more) else { return false };
                cost = cost.plus(&k);
                game.obj_mut(id).offspring = true;
            }
        }
    }
    // `handlePhyrexianManaCosts`: 2 life instead of each phyrexian symbol, asked before X and targets (the life is
    // paid with the mana)
    let Some(phyrexian_life) = ask_phyrexian(game, d, seat, &mut cost) else { return false };
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
        let possible =
            |game: &Game, k: usize| def.spell.modes[k].targets.iter().all(|t| target_possible(game, seat, t, Some(id)));
        if n == min {
            modes = (0..n).filter(|k| possible(game, *k)).collect();
        } else {
            while modes.len() < max {
                let available: Vec<usize> = (0..n).filter(|k| !modes.contains(k) && possible(game, *k)).collect();
                if available.is_empty() {
                    break;
                }
                match ask(game, d, seat, &Query::ChooseMode { modes: available.clone() }) {
                    Answer::Mode(k) if available.contains(&k) => {
                        modes.push(k);
                        // (the spell shows the modes chosen so far while the next is asked)
                        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                            top.modes = modes.clone();
                        }
                    }
                    Answer::ModeDone if modes.len() >= min => break,
                    Answer::Stop => return false,
                    _ => {
                        modes.clear();
                        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                            top.modes.clear();
                        }
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
    // spree: each chosen mode's cost
    for k in &modes {
        if let Some(more) = def.spell.modes.get(*k).and_then(|m| m.cost.as_ref()) {
            let Ok(more) = ManaCost::parse(more) else { return false };
            cost = cost.plus(&more);
        }
    }
    let mut defs: Vec<TargetDef> = Vec::new();
    let mut mode_ends: Vec<usize> = Vec::new();
    for k in if modes.is_empty() { vec![0] } else { modes.clone() } {
        defs.extend(def.spell.mode(k).1.iter().cloned());
        mode_ends.push(defs.len());
    }
    if let Some((unkicked, kicked)) = &def.spell.kicked_targets {
        let k = game.stack.last().is_some_and(|t| t.kicked);
        defs = if k { kicked.clone() } else { unkicked.clone() };
        mode_ends.clear();
    }
    if let Some((plain, promised)) = &def.spell.gift_targets {
        let g = game.stack.last().is_some_and(|t| t.gift_promised);
        defs = if g { promised.clone() } else { plain.clone() };
        mode_ends.clear();
    }
    // `handleOtherXCosts`: a `SacrificeXTargetCost`'s X is announced before targets (asked only with a choice)
    let mut sacrifice_x = None;
    for c in &def.spell.costs {
        if let CostDef::SacrificeXTargetCost { filter } = c {
            let max = matching(game, filter, seat, id).iter().filter(|p| game.obj(**p).controller == seat).count();
            let n = if max == 0 {
                0
            } else {
                let q = Query::Amount { min: 0, max: max as i32 };
                loop {
                    match ask(game, d, seat, &q) {
                        Answer::Int(n) if (0..=max as i32).contains(&n) => break n as usize,
                        Answer::Stop => return false,
                        _ => {}
                    }
                }
            };
            sacrifice_x = Some((filter.clone(), n));
        }
    }
    if !choose_targets_of_modes(game, d, seat, &defs, &mode_ends, Some(id), &def.spell.rule, true) {
        return false;
    }
    for s in 0..game.seats() {
        game.player_mut(s).passed = false;
    }
    let mut cost = modified_cost(game, seat, id, cost);
    if let Some((_, n)) = &sacrifice_x {
        if sacrifice_x_reduction(game, seat, id, &def).is_some() {
            cost.generic = cost.generic.saturating_sub(*n as u32);
        }
    }
    // `ExileCardsFromHandAdjuster`: how many to exile (AMOUNT), each taking {4} off; exiled with the other costs
    let mut exile_from_hand = None;
    for a in &def.abilities {
        if let AbilityDef::ExileCardsFromHandAdjusterAbility { filter, .. } = a {
            let cards: Vec<ObjId> = game
                .player(seat)
                .hand
                .iter()
                .copied()
                .filter(|c| game.filter_matches(filter, seat, Some(id), *c))
                .collect();
            if cards.is_empty() {
                continue;
            }
            let q = Query::Amount { min: 0, max: cards.len() as i32 };
            let n = loop {
                match ask(game, d, seat, &q) {
                    Answer::Int(n) if (0..=cards.len() as i32).contains(&n) => break n as usize,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            cost.generic = cost.generic.saturating_sub(4 * n as u32);
            if n > 0 {
                exile_from_hand = Some((cards, n));
            }
        }
    }
    // `HarmonizeAbilityAdjuster`: tap a creature to reduce the cost by its power
    if def.spell.costs.contains(&CostDef::HarmonizeCost) {
        let creatures = untapped_creatures(game, seat);
        if !creatures.is_empty() {
            let q = Query::YesNo { message: "Tap a creature to reduce the cost of this spell?".into() };
            let yes = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return false,
                    _ => {}
                }
            };
            if yes {
                // `tryToAutoChoose` reads the spell's text (Winternight Stories' "discard" isn't auto-chosen)
                let rule = def.spell.rule.clone();
                let Some(chosen) = choose_cards(game, d, seat, &creatures, &creatures, 1, 1, &rule) else {
                    return false;
                };
                if let Some(c) = chosen.first().copied() {
                    let power = game.power_toughness(c).0.max(0) as u32;
                    cost.generic = cost.generic.saturating_sub(power);
                    game.obj_mut(id).harmonize_tap = Some(c);
                }
            }
        }
    }
    game.paying_for.set(Some(id));
    let tally = game.spent_tally;
    if phyrexian_life > 0 {
        lose_life(game, seat, phyrexian_life);
    }
    let paid = pay_mana(game, d, seat, &cost);
    game.paying_for.set(None);
    if !paid {
        return false;
    }
    game.mana_paid = game.unpaid;
    game.obj_mut(id).mana_spent =
        cost.generic + cost.colored.iter().sum::<u32>() + cost.colorless + cost.hybrid.len() as u32;
    let mut spent = [0; 5];
    for (k, n) in spent.iter_mut().enumerate() {
        *n = game.spent_tally[k] - tally[k];
    }
    game.obj_mut(id).spent_colors = spent;
    // expend: the spell's mana is counted one by one; the 4th, 6th or 8th fires "whenever you expend N"
    let before = game.player(seat).expended;
    let after = before + (0..6).map(|k| game.spent_tally[k] - tally[k]).sum::<u32>();
    game.player_mut(seat).expended = after;
    if after > before {
        let sources = game.trigger_sources();
        let fired = game.triggers(
            sources,
            |_, _, you, t| matches!(t, Trigger::Expend { n } if you == seat && before < *n && *n <= after),
        );
        game.pending.extend(fired);
    }
    // 601.2g: then the other costs
    for c in &alternative_costs {
        if !pay_cost(game, d, seat, id, c) {
            return false;
        }
    }
    for c in &def.spell.costs {
        if let (CostDef::SacrificeXTargetCost { .. }, Some((filter, n))) = (c, &sacrifice_x) {
            if *n > 0
                && !pay_cost(game, d, seat, id, &CostDef::SacrificeTargetCost { filter: filter.clone(), amount: *n })
            {
                return false;
            }
            continue;
        }
        if !pay_cost(game, d, seat, id, c) {
            return false;
        }
    }
    if let Some((cards, n)) = exile_from_hand {
        let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Hand).collect();
        let Some(chosen) = choose_cards(game, d, seat, &cards, &cards, n, n, "exile") else {
            return false;
        };
        if chosen.len() < n {
            return false;
        }
        for c in chosen {
            game.move_to(c, Zone::Exile);
        }
    }
    for c in casualty.iter().chain(&teamwork).chain(&bargain) {
        if !pay_cost(game, d, seat, id, c) {
            return false;
        }
    }
    // `BlightCost.doBlight`: the counters go on a creature the player chooses
    if let Some(n) = blight {
        let creatures = controlled_creatures(game, seat);
        let Some(chosen) = choose_cards(game, d, seat, &creatures, &creatures, 1, 1, &def.spell.rule) else {
            return false;
        };
        match chosen.first() {
            Some(c) => game.obj_mut(*c).add_counters("-1/-1", n),
            None => return false,
        }
    }
    // `PromiseGiftCost.pay`: the opponent it's promised to (a player is never auto-chosen)
    if game.stack.last().is_some_and(|t| t.gift_promised) {
        let Some(chosen) = choose_target(game, d, seat, &TargetDef::TargetOpponent, Some(id), "", false) else {
            return false;
        };
        let to = match chosen.first() {
            Some(Target::Player(p)) => Some(*p),
            _ => None,
        };
        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
            top.gift_to = to;
        }
    }
    // `Spell.activate`: each spliced ability is activated after the spell's own (its targets; its cost was added)
    let spliced: Vec<ObjId> =
        game.stack.last().map(|t| t.spliced.iter().map(|(c, _, _)| *c).collect()).unwrap_or_default();
    for (k, c) in spliced.into_iter().enumerate() {
        let splice_def = game.obj(c).def.clone();
        let Some(top) = game.stack.last() else { return false };
        let (own, own_zcc) = (top.targets.clone(), top.target_zcc.clone());
        if !choose_targets(game, d, seat, &splice_def.spell.targets, Some(id), &splice_def.spell.rule, true) {
            return false;
        }
        let top = Arc::make_mut(&mut game.stack).last_mut().unwrap();
        let ts = std::mem::replace(&mut top.targets, own);
        let zs = std::mem::replace(&mut top.target_zcc, own_zcc);
        top.spliced[k].1 = ts;
        top.spliced[k].2 = zs;
    }
    if log::kept(game) {
        let line = log::casts(game, seat, id);
        log::inform(game, line);
    }
    // SPELL_CAST
    let spell = id;
    {
        let instant_or_sorcery = def.is(CardType::Instant) || def.is(CardType::Sorcery);
        let flier = def.is(CardType::Creature)
            && !def.subtypes.iter().any(|s| s == "Lemur")
            && game.has_ability(spell, &AbilityDef::FlyingAbility);
        let p = game.player_mut(seat);
        p.flier_cast |= flier;
        p.spells_cast += 1;
        if !def.is(CardType::Creature) {
            p.noncreature_cast += 1;
        }
        if instant_or_sorcery {
            p.instants_sorceries_cast += 1;
        }
    }
    note(game, EventKind::Cast, Some(seat), None, Some(Target::Obj(spell)), 0);
    // TARGETED's valiant, ward and Surrak triggers: `processAction` handles them before SPELL_CAST
    ward(game);
    let sources: Vec<ObjId> = game.trigger_sources();
    let targeted: Vec<ObjId> = game
        .stack
        .last()
        .map(|t| {
            t.targets.iter().flatten().filter_map(|x| if let Target::Obj(o) = x { Some(*o) } else { None }).collect()
        })
        .unwrap_or_default();
    // the rest of TARGETED
    let mut fired = game.triggers(sources.clone(), |g, src, you, t| match t {
        Trigger::BecomesTargetOfSpell { filter } => {
            targeted.contains(&src) && g.filter_matches(filter, you, Some(src), spell)
        }
        _ => false,
    });
    for t in fired.iter_mut() {
        if set_target_pointer(game, t) {
            t.targets = vec![vec![Target::Player(seat)]];
            t.target_zcc = vec![vec![0]];
        }
    }
    game.pending.extend(fired);
    // delayed triggers first: "when you next cast a creature spell this turn"
    if game.obj(spell).is(CardType::Creature) {
        let turn = game.turn;
        let (now, rest): (Vec<_>, Vec<_>) =
            std::mem::take(&mut game.next_creature_spell).into_iter().partition(|(s, t, _)| *s == seat && *t == turn);
        game.next_creature_spell = rest.into_iter().filter(|(_, t, _)| *t == turn).collect();
        for (_, _, mut item) in now {
            if let Some(dl) = item.delayed.as_mut() {
                Arc::make_mut(dl).fixed = vec![(spell, game.obj(spell).zcc)];
            }
            game.pending.push(item);
        }
    }
    // Galvanic Iteration: the next instant or sorcery spell this turn
    if game.obj(spell).is(CardType::Instant) || game.obj(spell).is(CardType::Sorcery) {
        let turn = game.turn;
        let (now, rest): (Vec<_>, Vec<_>) =
            std::mem::take(&mut game.next_instant_sorcery).into_iter().partition(|(s, t, _)| *s == seat && *t == turn);
        game.next_instant_sorcery = rest.into_iter().filter(|(_, t, _)| *t == turn).collect();
        for (_, _, mut item) in now {
            if let Some(dl) = item.delayed.as_mut() {
                Arc::make_mut(dl).fixed = vec![(spell, game.obj(spell).zcc)];
            }
            game.pending.push(item);
        }
    }
    let crime = game
        .stack
        .iter()
        .rev()
        .find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none())
        .is_some_and(|i| is_crime(game, seat, &i.targets));
    let fired = game.triggers(sources.clone(), |g, src, you, t| match t {
        Trigger::SpellCast { filter, who } => {
            Game::whose(who, you, seat) && g.filter_matches(filter, you, Some(src), spell)
        }
        Trigger::OpponentCastsOnYourTurnOrDies => {
            you != seat && g.active == Some(you) && g.obj(src).zone == Zone::Battlefield
        }
        Trigger::CommittedCrime => you == seat && crime,
        Trigger::SpellCastTargetingThis => you == seat && targeted.contains(&src),
        Trigger::CastSecondSpell { who } => Game::whose(who, you, seat) && g.player(seat).spells_cast == 2,
        _ => false,
    });
    let mut fired = fired;
    // "when you cast this spell"
    let replicated = game
        .stack
        .iter()
        .rev()
        .find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none())
        .map_or(0, |i| i.replicated);
    fired.extend(game.triggers(
        [spell],
        |_, src, _, t| matches!(t, Trigger::CastThis { replicated: r } if src == spell && (!r || replicated > 0)),
    ));
    fired.sort_by_key(|t| game.trigger_key(t));
    // the spell's mana value, X included (Shark Typhoon's "that spell's mana value")
    let spell_mv = {
        let c = game.obj(spell).def.printed();
        c.mana_value() as i32 + game.stack.last().map_or(0, |t| t.x) * c.x as i32
    };
    let opus = game.obj(spell).mana_spent >= 5;
    let spell_zcc = game.obj(spell).zcc;
    let spell_kicked = game
        .stack
        .iter()
        .rev()
        .find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none())
        .is_some_and(|i| i.kicked);
    for t in fired.iter_mut() {
        t.x = spell_mv;
        // a cast trigger of the spell itself: whether it was kicked (Sowing Mycospawn); others: "five or more mana"
        let has_kicker = game.obj(spell).def.abilities.iter().any(|a| matches!(a, AbilityDef::KickerAbility { .. }));
        t.kicked = if t.card == spell && has_kicker { spell_kicked } else { opus };
        let a = t.ability.and_then(|(i, _)| game.item_def(t).abilities.get(i));
        let to_player = matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. }));
        let to_spell = matches!(
            a,
            Some(AbilityDef::TriggeredAbility { on: Trigger::SpellCast { .. }, set_target_pointer: true, .. })
        );
        if to_player {
            t.targets = vec![vec![Target::Player(seat)]];
            t.target_zcc = vec![vec![0]];
        }
        // `SetTargetPointer.SPELL`: "that spell"
        if to_spell {
            t.targets = vec![vec![Target::Obj(spell)]];
            t.target_zcc = vec![vec![spell_zcc]];
        }
    }
    game.pending.extend(fired);
    true
}

/// `ContinuousEffects.applySpliceEffects`: the cards in the caster's hand that can be spliced onto this spell (their
/// subtype matches, and their spell ability's targets can be chosen); asked "Splice a card?", then a card at a time
/// ("Splice another card?" while others are left). The spliced cards' costs, added to the spell's. None: stopped.
fn choose_splices(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, def: &CardDef) -> Option<ManaCost> {
    let mut total = ManaCost::default();
    let mut left: Vec<(ObjId, String)> = game
        .player(seat)
        .hand
        .iter()
        .copied()
        .filter(|c| *c != id)
        .filter_map(|c| {
            let card = &game.obj(c).def;
            card.abilities.iter().find_map(|a| match a {
                AbilityDef::SpliceAbility { subtype, cost, .. }
                    if def.has_subtype(subtype)
                        && card.spell.targets.iter().all(|t| target_possible(game, seat, t, Some(id))) =>
                {
                    Some((c, cost.clone()))
                }
                _ => None,
            })
        })
        .collect();
    if left.is_empty() {
        return Some(total);
    }
    let yes = |game: &mut Game, d: &mut dyn Decider, message: &str| loop {
        match ask(game, d, seat, &Query::YesNo { message: message.into() }) {
            Answer::Bool(b) => break Some(b),
            Answer::Stop => break None,
            _ => {}
        }
    };
    if !yes(game, d, "Splice a card?")? {
        return Some(total);
    }
    loop {
        let cards: Vec<ObjId> = left.iter().map(|(c, _)| *c).collect();
        // (`tryToAutoChoose` reads the spell's text: Goryo's Vengeance's "exile" keeps a card of yours from being
        // chosen by itself)
        let chosen = choose_cards(game, d, seat, &cards, &cards, 1, 1, &def.spell.rule)?;
        if let Some(c) = chosen.first().copied() {
            if let Some(k) = left.iter().position(|(o, _)| *o == c) {
                let (_, cost) = left.remove(k);
                total = total.plus(&ManaCost::parse(&cost).ok()?);
                if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                    top.spliced.push((c, Vec::new(), Vec::new()));
                }
            }
        }
        if left.is_empty() || !yes(game, d, "Splice another card?")? {
            break;
        }
    }
    Some(total)
}

/// `CantCastMoreThanOneSpellEffect`: the player already cast a spell this turn.
fn controlled_creatures(game: &Game, seat: Seat) -> Vec<ObjId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|c| game.obj(*c).controller == seat && game.has_type(*c, CardType::Creature))
        .collect()
}

/// `CommittedCrimeTriggeredAbility.getCriminal`: `seat`'s spell or ability targets an opponent, a permanent, spell or
/// ability an opponent controls, or a card in an opponent's graveyard
fn is_crime(game: &Game, seat: Seat, targets: &[Vec<Target>]) -> bool {
    targets.iter().flatten().any(|t| match t {
        Target::Player(p) => *p != seat,
        Target::Obj(o) => {
            let x = game.obj(*o);
            match x.zone {
                Zone::Battlefield | Zone::Stack => x.controller != seat,
                Zone::Graveyard => x.owner != seat,
                _ => false,
            }
        }
    })
}

/// "Whenever you commit a crime" triggers, for the top of the stack (a spell just cast, an ability just activated or
/// put on the stack)
fn commit_crime(game: &mut Game, seat: Seat) {
    let Some(top) = game.stack.last() else { return };
    if top.controller != seat || !is_crime(game, seat, &top.targets) {
        return;
    }
    let sources = game.trigger_sources();
    let fired = game.triggers(sources, |_, _, you, t| matches!(t, Trigger::CommittedCrime) && you == seat);
    game.pending.extend(fired);
}

fn untapped_creatures(game: &Game, seat: Seat) -> Vec<ObjId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|c| game.obj(*c).controller == seat && game.has_type(*c, CardType::Creature) && !game.obj(*c).tapped)
        .collect()
}

fn cant_cast_more(game: &Game, seat: Seat) -> bool {
    game.player(seat).spells_cast >= 1
        && game.any_static(|g, src, e| match e {
            EffectDef::CantCastMoreThanOneSpellEffect { who } => Game::whose(who, g.obj(src).controller, seat),
            _ => false,
        })
        || game.any_static(|g, src, e| {
            matches!(e, EffectDef::CantCastDuringYourTurnEffect)
                && Some(g.obj(src).controller) == g.active
                && g.obj(src).controller != seat
        })
}

/// `ContinuousEffects.costModification`: a spell's cost with the increases, then the reductions, applied.
/// Bargain's cost: sacrifice an artifact, enchantment, or token
fn bargain_cost() -> CostDef {
    let filter = crate::card::Filter(vec![Pred::Or {
        xs: vec![
            Pred::Type { t: CardType::Artifact },
            Pred::Type { t: CardType::Enchantment },
            Pred::Token { v: true },
        ],
    }]);
    CostDef::SacrificeTargetCost { filter, amount: 1 }
}

fn modified_cost(game: &Game, seat: Seat, id: ObjId, cost: ManaCost) -> ManaCost {
    modified_cost_of(game, seat, id, &game.obj(id).def, cost)
}

/// Whether a spell filter matches `id` cast as `face` (an adventurer card's adventure, an instant, is checked as
/// one: Artist's Talent makes Free the Fae cheaper, not Picklock Prankster): its types from the face, the rest
/// from the card
fn face_matches(game: &Game, filter: &crate::card::Filter, you: Seat, src: ObjId, id: ObjId, face: &CardDef) -> bool {
    fn holds(game: &Game, p: &Pred, you: Seat, src: ObjId, id: ObjId, face: &CardDef) -> bool {
        match p {
            Pred::Type { t } => face.is(*t),
            Pred::Not { x } => !holds(game, x, you, src, id, face),
            Pred::Or { xs } => xs.iter().any(|x| holds(game, x, you, src, id, face)),
            Pred::And { xs } => xs.iter().all(|x| holds(game, x, you, src, id, face)),
            p => game.filter_matches(&crate::card::Filter(vec![p.clone()]), you, Some(src), id),
        }
    }
    if std::ptr::eq(face, &*game.obj(id).def) {
        return game.filter_matches(filter, you, Some(src), id);
    }
    filter.0.iter().all(|p| holds(game, p, you, src, id, face))
}

fn modified_cost_of(game: &Game, seat: Seat, id: ObjId, face: &CardDef, mut cost: ManaCost) -> ManaCost {
    cost.generic += game.obj(id).play_tax;
    // reductions after increases (`CostModificationType`)
    let mut reduce = 0;
    for src in game.battlefield.iter() {
        let s = game.obj(*src);
        for a in &s.def.abilities {
            if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                for e in effects {
                    if let EffectDef::SpellsCostIncreasingAllEffect { amount, filter, who } = e {
                        if Game::whose(who, s.controller, seat)
                            && face_matches(game, filter, s.controller, *src, id, face)
                        {
                            cost.generic += *amount as u32;
                        }
                    }
                    if let EffectDef::SpellsCostReductionControllerEffect { amount, filter, condition } = e {
                        if s.controller == seat
                            && face_matches(game, filter, seat, *src, id, face)
                            && game.condition_holds(condition.as_ref(), seat, *src)
                        {
                            reduce += *amount as u32;
                        }
                    }
                }
            }
        }
    }
    cost.generic = cost.generic.saturating_sub(reduce);
    let o = game.obj(id);
    for a in &o.def.abilities {
        // Ice Out: less if bargained (while checking what's playable: if it could be)
        if let AbilityDef::BargainAbility { discount, .. } = a {
            let bargained = if o.zone == Zone::Stack {
                game.stack.iter().rev().find(|t| t.card == id && t.ability.is_none()).is_some_and(|t| t.kicked)
            } else {
                can_pay_cost(game, seat, id, &bargain_cost())
            };
            if bargained {
                cost.generic = cost.generic.saturating_sub(*discount);
            }
        }
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
    // "mana of any type can be spent to cast it" (Cruelclaw's Heist): its colored symbols take any mana
    if o.any_mana {
        cost.generic += cost.colored.iter().sum::<u32>() + cost.colorless;
        cost.colored = [0; 5];
        cost.colorless = 0;
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
    choose_targets_of_modes(game, d, seat, defs, &[], source, rule, cancellable)
}

/// `choose_targets`, for a spell's chosen modes: `mode_ends` are where each mode's targets end in `defs`
/// (`AbilityImpl.activate`: a mode whose targets are cancelled stops the cast before the next mode's are asked)
#[allow(clippy::too_many_arguments)]
fn choose_targets_of_modes(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    defs: &[TargetDef],
    mode_ends: &[usize],
    source: Option<ObjId>,
    rule: &str,
    cancellable: bool,
) -> bool {
    // `Targets.makeChoice`: a cancelled target doesn't stop the others being asked; the choice fails at the end
    let mut targets = Vec::new();
    let mut cancelled = false;
    for (i, t) in defs.iter().enumerate() {
        if cancelled && mode_ends.contains(&i) {
            return false;
        }
        if matches!(t, TargetDef::TargetAnyTarget { not_first: true, .. })
            || matches!(t, TargetDef::TargetPermanent { filter, .. } if filter.0.contains(&Pred::ControlledByFirstTarget))
        {
            game.chosen_before = targets.iter().flatten().copied().collect();
        }
        let chosen = choose_target(game, d, seat, t, source, rule, cancellable);
        game.chosen_before.clear();
        if let (
            Some(ts),
            TargetDef::TargetPermanent { distribute: Some(a), .. }
            | TargetDef::TargetAnyTarget { distribute: Some(a), .. },
        ) = (&chosen, t)
        {
            match distribute(game, d, seat, a, ts.len(), source) {
                Some(amounts) => {
                    if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                        top.target_amounts = amounts;
                    }
                }
                None if d.stopped() => return false,
                None => {}
            }
        }
        match chosen {
            Some(chosen) => targets.push(chosen),
            None if d.stopped() => return false,
            None => {
                cancelled = true;
                targets.push(Vec::new());
            }
        }
    }
    if cancelled {
        return false;
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

/// The total a divided target divides (the spell's X)
fn divided_total(game: &Game, a: &Amount, seat: Seat, source: Option<ObjId>) -> i32 {
    let x = game.stack.last().map(|t| t.x).unwrap_or(0);
    let src = source.or_else(|| game.stack.last().map(|t| t.card)).unwrap_or(ObjId(0));
    game.value(a, seat, src, x)
}

/// `HumanPlayer.chooseTargetAmount`'s second step: one target gets it all, as many targets as points one each, and
/// otherwise the player divides it (`getMultiAmount`, at least 1 each)
fn distribute(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    a: &Amount,
    n: usize,
    source: Option<ObjId>,
) -> Option<Vec<i32>> {
    let total = divided_total(game, a, seat, source);
    if n == 0 {
        return Some(Vec::new());
    }
    if n == 1 {
        return Some(vec![total]);
    }
    if n as i32 == total {
        return Some(vec![1; n]);
    }
    // `MultiAmountType.prepareDefaultValues`: 1 each, the rest on the first
    let mut defaults = vec![1; n];
    defaults[0] += total - n as i32;
    multi_amount(game, d, seat, defaults, vec![(1, total); n], (total, total), (None, Vec::new()))
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
    targeted(game, fired);
    let mut fired = Vec::new();
    for t in top.targets.iter().flatten() {
        let Target::Obj(o) = *t else { continue };
        let obj = game.obj(o);
        // Surrak: a creature or creature spell of yours becomes the target of an opponent's spell or ability
        let creature = (obj.zone == Zone::Battlefield && game.has_type(o, CardType::Creature))
            || (obj.zone == Zone::Stack && obj.is(CardType::Creature));
        if creature && obj.controller != top.controller {
            let you = obj.controller;
            let sources: Vec<ObjId> =
                game.battlefield.iter().copied().filter(|s| game.obj(*s).controller == you).collect();
            fired.extend(game.triggers(sources, |_, _, _, t| matches!(t, Trigger::SurrakTargeted)));
        }
        let obj = game.obj(o);
        if obj.zone != Zone::Battlefield || obj.controller == top.controller {
            continue;
        }
        if game.has_type(o, CardType::Creature) && nowhere_to_run(game, obj.controller) {
            continue;
        }
        // the ward abilities statics give it, after its own (on the stack with a face that has them)
        let given = game.granted_wards(o);
        let face = if given.is_empty() {
            None
        } else {
            let mut def = (*obj.def).clone();
            def.abilities.extend(given);
            Some(Arc::new(def))
        };
        let def = face.clone().unwrap_or_else(|| obj.def.clone());
        for (i, a) in def.abilities.iter().enumerate() {
            if let AbilityDef::WardAbility { .. } = a {
                let mut item =
                    StackItem { ability: Some((i, obj.zcc)), def: face.clone(), ..StackItem::spell(o, obj.controller) };
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
    // one event's triggers wait in registration order
    fired.sort_by_key(|t| game.trigger_key(t));
    targeted(game, fired);
}

/// What a TARGETED event triggered: waiting, or held while triggered abilities go on the stack
fn targeted(game: &mut Game, fired: Vec<StackItem>) {
    match game.targeted_later.as_mut() {
        Some(later) => later.extend(fired),
        None => game.pending.extend(fired),
    }
}
/// `PlayerImpl.playAbility`: the ability goes on the stack, then its targets, then mana, then other costs.
/// False: cancelled.
fn activate(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, index: usize) -> bool {
    let outer = game.unpaid.replace(game.stack.len());
    let done = activate_paying(game, d, seat, id, index);
    game.unpaid = outer;
    game.mana_paid = None;
    if done && log::kept(game) {
        if let Some(top) = game.stack.last().filter(|t| t.card == id && t.ability.is_some_and(|(k, _)| k == index)) {
            let line = log::activates(game, seat, top);
            log::inform(game, line);
        }
    }
    done
}

fn activate_paying(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, index: usize) -> bool {
    let def = game.obj(id).def.clone();
    let Some((rule, _, targets, costs, _)) = def.abilities[index].activated() else { return false };
    let zcc = game.obj(id).zcc;
    if let AbilityDef::LimitedTimesPerTurnActivatedAbility { .. } = def.abilities[index] {
        game.activated_this_turn.push((id, index));
    }
    if let AbilityDef::PowerUpAbility { .. } = def.abilities[index] {
        game.obj_mut(id).powered_up = true;
    }
    // the face it was activated from: an ability an Aura grants is gone from a land sacrificed to pay for it
    Arc::make_mut(&mut game.stack).push(StackItem {
        ability: Some((index, zcc)),
        def: Some(def.clone()),
        ..StackItem::spell(id, seat)
    });
    // X: announced before the targets (`AbilityImpl.handleManaXCosts`)
    let mut mana = adjusted_mana_cost(game, seat, id, &def.abilities[index], costs);
    if mana.x > 0 {
        let min = match def.abilities[index] {
            AbilityDef::SimpleActivatedAbility { min_x, .. } => min_x,
            _ => 0,
        };
        let q = Query::Amount { min, max: i32::MAX };
        let x = loop {
            match ask(game, d, seat, &q) {
                Answer::Int(n) if n >= min => break n,
                Answer::Stop => return false,
                _ => {}
            }
        };
        mana.generic += x as u32 * mana.x;
        mana.x = 0;
        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
            top.x = x;
        }
    }
    // "tap X untapped artifacts you control": X announced before the mana (`VariableCostImpl`), up to the ones
    // untapped now (the source too)
    if let Some(CostDef::TapXTargetCost { filter }) = costs.iter().find(|c| matches!(c, CostDef::TapXTargetCost { .. }))
    {
        let max = matching(game, filter, seat, id)
            .into_iter()
            .filter(|p| game.obj(*p).controller == seat && !game.obj(*p).tapped)
            .count() as i32;
        let x = loop {
            match ask(game, d, seat, &Query::Amount { min: 0, max }) {
                Answer::Int(n) if (0..=max).contains(&n) => break n,
                Answer::Stop => return false,
                _ => {}
            }
        };
        if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
            top.x = x;
        }
    }
    if !choose_targets(game, d, seat, targets, None, rule, true) {
        return false;
    }
    // an equip ability's reductions are for the creature it targets, now that it's chosen
    if let AbilityDef::EquipAbility { .. } = def.abilities[index] {
        if equip_discount(game, seat, None) > 0 {
            let first = game.stack.last().and_then(|t| t.targets.first()).and_then(|t| t.first()).copied();
            let off = match first {
                Some(Target::Obj(o)) => equip_discount(game, seat, Some(o)),
                _ => 0,
            };
            let own = adjusted_mana_cost_own(game, seat, id, &def.abilities[index], costs).generic;
            mana.generic = own.saturating_sub(off);
        }
    }
    if let AbilityDef::EquipAbility { colorless_discount, .. } = def.abilities[index] {
        if colorless_discount > 0 {
            let first = game.stack.last().and_then(|t| t.targets.first()).and_then(|t| t.first()).copied();
            let colorless = matches!(first, Some(Target::Obj(o)) if game.colors(o).is_empty());
            let discounted = ability_mana_cost(costs).generic - mana.generic;
            if !colorless && discounted > 0 {
                mana.generic += discounted;
            } else if colorless && discounted == 0 {
                mana.generic = mana.generic.saturating_sub(colorless_discount);
            }
        }
    }
    for s in 0..game.seats() {
        game.player_mut(s).passed = false;
    }
    // conditional mana for abilities (Sunken Citadel's) knows what it's paying for
    let paying = (game.paying_for.get(), game.paying_ability.get());
    game.paying_for.set(Some(id));
    game.paying_ability.set(true);
    let paid = pay_mana(game, d, seat, &mana);
    game.paying_for.set(paying.0);
    game.paying_ability.set(paying.1);
    if !paid {
        return false;
    }
    game.mana_paid = game.unpaid;
    for c in costs {
        if !matches!(c, CostDef::ManaCost { .. }) && !pay_cost(game, d, seat, id, c) {
            return false;
        }
    }
    // `ACTIVATED_ABILITY`: "when you cycle this card" (`CycleTriggeredAbility`), with the cycling's X
    if matches!(def.abilities[index], AbilityDef::CyclingAbility { .. }) {
        let x = game.stack.last().map_or(0, |t| t.x);
        let fired = game.triggers([id], |_, _, _, t| matches!(t, Trigger::Cycled));
        game.pending.extend(fired.into_iter().map(|t| StackItem { x, ..t }));
    }
    commit_crime(game, seat);
    true
}

/// `CounterUnlessPaysEffect`'s payment: `canPay`, `chooseUse` "Pay {2}?" (a mana cost's `canPay` is always true), then
/// the payment; anything short of paying counters. None: stopped.
fn pay_unless(game: &mut Game, d: &mut dyn Decider, payer: Seat, id: ObjId, cost: &[CostDef]) -> Option<bool> {
    // ward—discard a card: a non-mana cost's message has no "Pay " (`DiscardTargetCost` text "discard a card")
    if let Some(CostDef::DiscardCardCost { filter, .. }) =
        cost.iter().find(|c| matches!(c, CostDef::DiscardCardCost { .. }))
    {
        let hand = game.player(payer).hand.clone();
        let possible: Vec<ObjId> =
            hand.iter().copied().filter(|c| game.filter_matches(filter, payer, None, *c)).collect();
        if possible.is_empty() {
            return Some(false);
        }
        let q = Query::YesNo { message: "discard a card?".into() };
        let yes = loop {
            match ask(game, d, payer, &q) {
                Answer::Bool(b) => break b,
                Answer::Stop => return None,
                _ => {}
            }
        };
        if !yes {
            return Some(false);
        }
        let chosen = choose_cards(game, d, payer, &hand, &possible, 1, 1, "discard");
        if d.stopped() {
            return None;
        }
        let Some(chosen) = chosen else { return Some(false) };
        for o in &chosen {
            game.move_to(*o, Zone::Graveyard);
        }
        return Some(!chosen.is_empty());
    }
    // ward—sacrifice three nonland permanents: `canPay` first (too few: countered, nothing asked), then "<cost>?"
    if let Some(c @ CostDef::SacrificeTargetCost { amount, .. }) =
        cost.iter().find(|c| matches!(c, CostDef::SacrificeTargetCost { .. }))
    {
        if !can_pay_cost(game, payer, id, c) {
            return Some(false);
        }
        let q = Query::YesNo { message: format!("sacrifice {} permanents?", amount) };
        let yes = loop {
            match ask(game, d, payer, &q) {
                Answer::Bool(b) => break b,
                Answer::Stop => return None,
                _ => {}
            }
        };
        let paid = yes && pay_cost(game, d, payer, id, c);
        if d.stopped() {
            return None;
        }
        return Some(paid);
    }
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

/// `ReduceCostEquipTargetSourceEffect` (Fervent Champion): what `seat`'s equip abilities cost less for targeting
/// `only`, or, with none, for every creature of theirs it could target (`inCheckPlayableState`: the first target can
/// be it)
fn equip_discount(game: &Game, seat: Seat, only: Option<ObjId>) -> u32 {
    let mut n = 0;
    for src in game.battlefield.iter() {
        if only.is_some_and(|o| o != *src)
            || game.obj(*src).controller != seat
            || !game.has_type(*src, CardType::Creature)
        {
            continue;
        }
        for a in game.obj(*src).def.abilities.iter() {
            if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                for e in effects {
                    if let EffectDef::ReduceCostEquipTargetSourceEffect { amount } = e {
                        n += amount;
                    }
                }
            }
        }
    }
    n
}

/// An activated ability's mana cost, after its cost adjuster (Channel's legendary discount) and, for an equip
/// ability, the playable check's reductions for what it could target
fn adjusted_mana_cost(game: &Game, seat: Seat, id: ObjId, a: &AbilityDef, costs: &[CostDef]) -> ManaCost {
    let mut cost = adjusted_mana_cost_own(game, seat, id, a, costs);
    if let AbilityDef::EquipAbility { .. } = a {
        cost.generic = cost.generic.saturating_sub(equip_discount(game, seat, None));
    }
    cost
}

fn adjusted_mana_cost_own(game: &Game, seat: Seat, id: ObjId, a: &AbilityDef, costs: &[CostDef]) -> ManaCost {
    let mut cost = ability_mana_cost(costs);
    // power-up the turn it entered: less its own mana cost
    if matches!(a, AbilityDef::PowerUpAbility { .. }) && game.obj(id).entered_turn == game.turn {
        let own = &game.obj(id).def.cost;
        for k in 0..5 {
            cost.colored[k] = cost.colored[k].saturating_sub(own.colored[k]);
        }
        cost.generic = cost.generic.saturating_sub(own.generic);
    }
    // Ghostfire Blade: {2} less if it can target a colorless creature (`inCheckPlayableState`); `activate` adds it
    // back once the target turns out not to be colorless
    if let AbilityDef::EquipAbility { colorless_discount, .. } = a {
        if *colorless_discount > 0
            && game.battlefield.iter().any(|b| {
                let o = game.obj(*b);
                o.controller == seat && game.has_type(*b, CardType::Creature) && game.colors(*b).is_empty()
            })
        {
            cost.generic = cost.generic.saturating_sub(*colorless_discount);
        }
    }
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
        CostDef::ExertSourceCost => game.obj_mut(id).exerted = true,
        // the `TapTargetCost` for the creature chosen as the cost was reduced: chosen again (the ability's text
        // says "exile", so it isn't chosen by itself)
        CostDef::HarmonizeCost => {
            let Some(c) = game.obj_mut(id).harmonize_tap.take() else { return true };
            if game.obj(c).zone != Zone::Battlefield || game.obj(c).tapped {
                return false;
            }
            let rule = game.obj(id).def.spell.rule.clone();
            // "done" instead leaves the cost unpaid: the cast is rolled back
            if choose_cards_with(game, d, seat, &[c], &[c], 1, 1, &rule, true) != Some(vec![c]) {
                return false;
            }
            tap(game, c);
        }
        CostDef::TapXTargetCost { filter } => {
            let can: Vec<ObjId> = matching(game, filter, seat, id)
                .into_iter()
                .filter(|p| game.obj(*p).controller == seat && !game.obj(*p).tapped)
                .collect();
            let x = game.stack.last().map_or(0, |t| t.x).max(0) as usize;
            if x > 0 {
                // not required: fewer than X chosen, the cost isn't paid and the activation is rolled back
                let rule = "untapped artifacts you control".to_string();
                let Some(chosen) = choose_cards_with(game, d, seat, &can, &can, x, x, &rule, true) else {
                    return false;
                };
                if chosen.len() < x {
                    return false;
                }
                for c in chosen {
                    tap(game, c);
                }
            }
        }
        // tapped already (it paid mana for this ability): the activation fails and is rolled back
        CostDef::TapSourceCost => {
            // gone already (a mana ability paying for it sacrificed it): the activation fails and is rolled back
            if game.obj(id).tapped || game.obj(id).zone != Zone::Battlefield {
                return false;
            }
            tap(game, id);
        }
        CostDef::SacrificeSourceCost if game.obj(id).zone != Zone::Battlefield => return false,
        CostDef::SacrificeSourceCost => {
            let power = game.power_toughness(id).0;
            if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                top.lki_power = Some(power);
            }
            sacrifice(game, id)
        }
        CostDef::ExileSourceFromGraveCost => {
            if game.obj(id).zone != Zone::Graveyard {
                return false;
            }
            game.move_to(id, Zone::Exile);
        }
        CostDef::ExileSourceCost => {
            let power = game.power_toughness(id).0;
            if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                top.lki_power = Some(power);
            }
            game.move_to(id, Zone::Exile);
        }
        // `RemoveCounterCost.pay`: the permanents, then from each with the counters, how many (asked with a choice)
        CostDef::RemoveCountersFromAmongCost { filter, counter, amount, max } => {
            let possible: Vec<ObjId> =
                matching(game, filter, seat, id).into_iter().filter(|p| game.obj(*p).controller == seat).collect();
            let Some(chosen) = choose_in_rounds(game, d, seat, &possible, 1, *max) else {
                return false;
            };
            let mut removed = 0;
            for c in chosen {
                let on = game.obj(c).counter(counter);
                if on <= 0 {
                    continue;
                }
                let left = amount - removed;
                let n = if left > 1 && on > 1 {
                    let hi = left.min(on);
                    let q = Query::Amount { min: 1, max: hi };
                    loop {
                        match ask(game, d, seat, &q) {
                            Answer::Int(n) if (1..=hi).contains(&n) => break n,
                            Answer::Stop => return false,
                            _ => {}
                        }
                    }
                } else {
                    1
                };
                game.obj_mut(c).add_counters(counter, -n);
                removed += n;
                if removed == *amount {
                    break;
                }
            }
            if removed < *amount {
                return false;
            }
        }
        CostDef::PutCountersSourceCost { counter } => game.obj_mut(id).add_counters(&counter.name, counter.amount),
        CostDef::RemoveCountersSourceCost { counter } => {
            let o = game.obj_mut(id);
            if o.counter(&counter.name) < counter.amount {
                return false;
            }
            o.add_counters(&counter.name, -counter.amount);
        }
        CostDef::PayLoyaltyCost { amount } => {
            let o = game.obj_mut(id);
            o.add_counters("loyalty", *amount);
            o.loyalty_used = true;
            note(game, EventKind::Loyalty, Some(seat), Some(id), None, 0);
        }
        CostDef::TapTargetCost { filter, amount } => {
            let possible: Vec<ObjId> =
                matching(game, filter, seat, id).into_iter().filter(|p| game.obj(*p).controller == seat).collect();
            // `TapTargetCost`: its target isn't required: done before enough are chosen cancels the ability
            // (Shacklegeist with one other Spirit)
            let Some(chosen) = choose_cards_with(game, d, seat, &possible, &possible, *amount, *amount, "tap", true)
            else {
                return false;
            };
            if chosen.len() < *amount {
                return false;
            }
            for o in chosen {
                tap(game, o);
            }
        }
        CostDef::CrewCost { value } => {
            let possible = crew_candidates(game, seat, id);
            let Some(mut chosen) = choose_cards(game, d, seat, &possible, &possible, 0, usize::MAX, "tap") else {
                return false;
            };
            // `TargetImpl.choose`: a round that ended with Done after choosing something asks again
            while !chosen.is_empty() && chosen.len() < possible.len() {
                let left: Vec<ObjId> = possible.iter().copied().filter(|c| !chosen.contains(c)).collect();
                let Some(more) = choose_cards(game, d, seat, &left, &left, 0, usize::MAX, "tap") else {
                    return false;
                };
                if more.is_empty() {
                    break;
                }
                chosen.extend(more);
            }
            let mut power = 0;
            let mut crew = Vec::new();
            for o in chosen {
                if !game.obj(o).tapped {
                    tap(game, o);
                    power += game.power_toughness(o).0;
                    crew.push(o);
                }
            }
            if power < *value {
                return false;
            }
            // "whenever this crews a Vehicle": that Vehicle is its target
            let zcc = game.obj(id).zcc;
            for o in crew {
                let mut fired = game.triggers([o], |_, _, _, t| matches!(t, Trigger::CrewsVehicle));
                for t in fired.iter_mut() {
                    t.targets = vec![vec![Target::Obj(id)]];
                    t.target_zcc = vec![vec![zcc]];
                }
                game.pending.extend(fired);
            }
        }
        CostDef::SacrificeTargetCost { filter, amount } => {
            let possible: Vec<ObjId> =
                matching(game, filter, seat, id).into_iter().filter(|p| game.obj(*p).controller == seat).collect();
            // not enough left (the mana paid first used them): `canPay` fails, nothing asked
            if possible.len() < *amount {
                return false;
            }
            // `SacrificeTargetCost`: its target can be cancelled
            let rule = if game.foraging.get() { "forage" } else { "sacrifice" };
            let Some(chosen) = choose_cards_with(game, d, seat, &possible, &possible, *amount, *amount, rule, true)
            else {
                return false;
            };
            if chosen.len() < *amount {
                return false;
            }
            // sacrificed together: their dies triggers see each other die
            let outer = std::mem::replace(&mut game.lookback, game.battlefield.to_vec());
            let toughness = chosen.iter().map(|o| game.power_toughness(*o).1).max();
            if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
                top.sacrificed_toughness = toughness;
            }
            let insert = game.sac_insert.replace(game.pending.len());
            for o in chosen {
                sacrifice(game, o);
            }
            game.sac_insert = insert;
            game.lookback = outer;
        }
        // announced and paid as a `SacrificeTargetCost` in `cast_with`
        CostDef::SacrificeXTargetCost { .. } => {}
        CostDef::DiscardCardCost { filter, amount } => {
            let hand = game.player(seat).hand.clone();
            let possible: Vec<ObjId> =
                hand.iter().copied().filter(|c| game.filter_matches(filter, seat, Some(id), *c)).collect();
            // checked as playable, the card counted itself; cast, it may leave nothing to discard (the cast is undone)
            if possible.len() < *amount {
                return false;
            }
            let Some(chosen) = choose_cards(game, d, seat, &hand, &possible, *amount, *amount, "discard") else {
                return false;
            };
            for o in chosen {
                game.move_to(o, Zone::Graveyard);
            }
        }
        CostDef::PayLifeCost { amount } => lose_life(game, seat, *amount),
        CostDef::PayEnergyCost { amount } => add_energy(game, seat, -*amount),
        CostDef::RevealTargetFromHandCost { filter, min, max } => {
            let hand = game.player(seat).hand.clone();
            let possible: Vec<ObjId> =
                hand.iter().copied().filter(|c| *c != id && game.filter_matches(filter, seat, Some(id), *c)).collect();
            let Some(chosen) = choose_cards_rounds(game, d, seat, &possible, *min, *max) else {
                return false;
            };
            if chosen.len() < *min {
                return false;
            }
            if !chosen.is_empty() {
                let zcc = game.obj(id).zcc;
                let cards = chosen.iter().map(|c| (*c, Zone::Hand)).collect();
                game.revealed.push(crate::state::Revealed { source: id, zcc, paren: false, cards });
            }
            // the number revealed: the ability's X
            if let Some(top) = Arc::make_mut(&mut game.stack).last_mut().filter(|t| t.card == id) {
                top.x = chosen.len() as i32;
            }
        }
        CostDef::ExileFromHandCost { filter } => {
            let hand = game.player(seat).hand.clone();
            let possible: Vec<ObjId> =
                hand.iter().copied().filter(|c| *c != id && game.filter_matches(filter, seat, Some(id), *c)).collect();
            let Some(chosen) = choose_cards(game, d, seat, &hand, &possible, 1, 1, "exile") else {
                return false;
            };
            if chosen.is_empty() {
                return false;
            }
            for o in chosen {
                game.move_to(o, Zone::Exile);
            }
        }
        // `BeholdType.beholdOne`
        CostDef::BeholdCost { subtype } => {
            let (perms, cards) = beholdable(game, seat, subtype);
            let use_permanent = match (perms.is_empty(), cards.is_empty()) {
                (true, true) => return false,
                (false, true) => true,
                (true, false) => false,
                (false, false) => {
                    let q = Query::YesNo {
                        message: format!("Choose a {} you control or reveal one from your hand?", subtype),
                    };
                    loop {
                        match ask(game, d, seat, &q) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return false,
                            _ => {}
                        }
                    }
                }
            };
            if use_permanent {
                let def = TargetDef::TargetPermanent {
                    filter: crate::card::Filter(vec![
                        Pred::Subtype { s: subtype.clone() },
                        Pred::Controller { who: "you".into() },
                    ]),
                    min: 1,
                    max: 1,
                    distribute: None,
                };
                // `tryToAutoChoose` reads the ability's text: an effect that searches too (Elven Passage) asks
                let card = &game.obj(id).def;
                let text = card
                    .abilities
                    .iter()
                    .filter_map(|a| a.activated())
                    .map(|a| a.0.to_lowercase())
                    .find(|r| r.contains("behold"))
                    .unwrap_or_else(|| card.spell.rule.clone());
                let rule = if text.contains("search") { text } else { "behold".to_string() };
                match choose_target(game, d, seat, &def, Some(id), &rule, false) {
                    Some(t) if !t.is_empty() => {}
                    _ => return false,
                }
            } else {
                let hand = game.player(seat).hand.clone();
                match choose_cards(game, d, seat, &hand, &cards, 1, 1, "behold") {
                    Some(c) if !c.is_empty() => {}
                    _ => return false,
                }
            }
        }
        CostDef::ReturnAttackerToHandTargetCost => {
            let possible = unblocked_attackers(game, seat);
            let rule = game.obj(id).def.abilities.iter().find_map(|a| match a {
                AbilityDef::NinjutsuAbility { rule, .. } => Some(rule.clone()),
                _ => None,
            });
            let Some(chosen) = choose_cards(game, d, seat, &possible, &possible, 1, 1, &rule.unwrap_or_default())
            else {
                return false;
            };
            let Some(&attacker) = chosen.first() else { return false };
            let defender = game.obj(attacker).attacking;
            let pw = game.obj(attacker).attacking_pw;
            game.obj_mut(id).ninjutsu_defender = defender;
            game.obj_mut(id).ninjutsu_pw = pw;
            game.move_to(attacker, Zone::Hand);
        }
        CostDef::ExileFromGraveCost { amount } => {
            let possible: Vec<ObjId> = game.player(seat).graveyard.iter().copied().filter(|c| *c != id).collect();
            let rule = game.obj(id).def.abilities.iter().find_map(|a| match a {
                AbilityDef::EscapeAbility { rule, .. } => Some(rule.clone()),
                _ => None,
            });
            let rule = if game.foraging.get() { Some("forage".to_string()) } else { rule };
            let Some(chosen) =
                choose_cards(game, d, seat, &possible, &possible, *amount, *amount, &rule.unwrap_or("exile".into()))
            else {
                return false;
            };
            if chosen.len() < *amount {
                return false;
            }
            for c in chosen {
                game.move_to(c, Zone::Exile);
            }
        }
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
        // `FORAGED` once it's paid
        CostDef::ForageCost => {
            game.foraging.set(true);
            let paid = pay_cost(game, d, seat, id, &forage_cost());
            game.foraging.set(false);
            if !paid {
                return false;
            }
            let sources = game.trigger_sources();
            let fired = game.triggers(sources, |_, _, you, t| matches!(t, Trigger::Forage) && you == seat);
            game.pending.extend(fired);
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
        discard_card(game, seat, *c);
    }
    chosen
}

/// `PlayerImpl.doDiscard`: the log first, then the card goes
fn discard_card(game: &mut Game, seat: Seat, c: ObjId) {
    if log::kept(game) {
        let mut line = format!("{} discards {}", log::player(seat), log::object(game, c));
        if let Some(s) = log::source(game) {
            line += &format!(" (source: {})", log::object(game, s));
        }
        log::inform(game, line);
    }
    note(game, EventKind::Discarded, Some(seat), None, Some(Target::Obj(c)), 0);
    game.move_to(c, Zone::Graveyard);
}

/// `PlayerImpl.searchLibrary` with `TargetCardInLibrary.choose`: the whole library is shown (sorted by name,
/// `Game::searching`) and the player picks cards of the filter (PICK_TARGET each), never automatically. Fewer than
/// the least is fine when fewer match; only a search for any card at all can't be left empty. None: stopped.
fn search_library(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    filter: &crate::card::Filter,
    min: usize,
    max: usize,
    source: ObjId,
) -> Option<Vec<ObjId>> {
    let mut shown = game.player(seat).library.clone();
    shown.sort_by(|a, b| game.obj(*a).name().cmp(game.obj(*b).name()));
    game.searching = Some((shown, Vec::new()));
    let chosen = search_library_cards(game, d, seat, filter, min, max, source);
    game.searching = None;
    chosen
}

fn search_library_cards(
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
            if let Some((_, so_far)) = &mut game.searching {
                so_far.clone_from(&chosen);
            }
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
            // `LibrarySearchedEvent`
            let sources = game.trigger_sources();
            let fired =
                game.triggers(sources, |_, _, you, t| matches!(t, Trigger::OpponentSearchesLibrary) && you != seat);
            game.pending.extend(fired);
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
    by_static
        || game.obj(spell).uncounterable
        || game.obj(spell).def.abilities.iter().any(|a| match a {
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
    let (min, mut max) = target_counts(def);
    if let TargetDef::TargetCardInGraveyard { max_x: true, .. } = def {
        let x = source
            .and_then(|s| game.stack.iter().rev().find(|i| i.card == s && i.ability.is_none()))
            .map_or(0, |i| i.x);
        // X of 0: no targets, and nothing asked (as Java's adjusted `TargetCardInGraveyard(0, 0)` plays)
        if x <= 0 {
            return Some(Vec::new());
        }
        max = x as usize;
    }
    // a divided target takes at most one target per point (`chooseTargetAmount`)
    if let TargetDef::TargetPermanent { distribute: Some(a), .. }
    | TargetDef::TargetAnyTarget { distribute: Some(a), .. } = def
    {
        max = max.min(divided_total(game, a, seat, source).max(0) as usize);
        // X divided: no fewest number of targets, so nothing to divide asks for none (Grove's Bounty for X=0)
        if max == 0 && matches!(a, crate::card::Amount::Of(_)) {
            return Some(Vec::new());
        }
    }
    let rule = rule.to_lowercase();
    let feel_bad = ["discard", "sacrifice", "destroy", "exile"].iter().any(|w| rule.contains(w));
    let mut chosen: Vec<Target> = Vec::new();
    let mut selected = false;
    let mut added_in_prompt = false;
    loop {
        let src = source.or_else(|| game.stack.last().map(|t| t.card));
        let (mut players, mut objs) = possible_targets(game, seat, def, src);
        // a spell can't target itself (rule 115.5)
        objs.retain(|o| Some(*o) != source);
        if let (TargetDef::TargetCardInGraveyard { single: true, .. }, Some(Target::Obj(first))) = (def, chosen.first())
        {
            let owner = game.obj(*first).owner;
            objs.retain(|o| game.obj(*o).owner == owner);
        }
        if chosen.is_empty() && min > 0 && players.is_empty() && objs.is_empty() {
            return None;
        }
        // `chooseTargetAmount`: a divided target is asked again after each one chosen, until "done"
        let divided = matches!(
            def,
            TargetDef::TargetPermanent { distribute: Some(_), .. }
                | TargetDef::TargetAnyTarget { distribute: Some(_), .. }
        );
        // `HumanPlayer.chooseTarget`: nothing to choose and nothing chosen returns without asking (an "up to" target
        // with no legal choice; `chooseTargetAmount`, for a divided one, still asks)
        if !divided && chosen.is_empty() && players.is_empty() && objs.is_empty() {
            return Some(chosen);
        }
        if chosen.len() >= max && !(divided && added_in_prompt) {
            return Some(chosen);
        }
        players.retain(|p| !chosen.contains(&Target::Player(*p)));
        objs.retain(|o| !chosen.contains(&Target::Obj(*o)));
        // a divided target with its most targets chosen is asked once more, but a new one isn't added
        // (Chandra's Pyrohelix: two damage, at most two targets)
        if divided && chosen.len() >= max {
            players.clear();
            objs.clear();
        }
        // `isChoiceCompleted`: nothing more to choose, but only once a target was chosen (a divided "up to" target
        // with nothing to choose is still asked once)
        if selected && players.is_empty() && objs.is_empty() && chosen.len() >= min && !(divided && added_in_prompt) {
            return Some(chosen);
        }
        if min == max && players.len() + objs.len() == min - chosen.len() && !rule.contains("search") {
            let auto = if game.player(seat).auto_target_all {
                players.first().map(|p| Target::Player(*p)).or_else(|| objs.first().map(|o| Target::Obj(*o)))
            } else {
                let own_player = players.iter().find(|p| **p == seat).map(|p| Target::Player(*p));
                // an ability on the stack is no permanent, card or spell: never chosen for them
                let ability = |o: ObjId| match def {
                    TargetDef::TargetStackAbility { .. } => true,
                    TargetDef::TargetTriggeredAbilityOrColorlessSpell => game.obj(o).zone != Zone::Stack,
                    _ => false,
                };
                own_player.filter(|_| !feel_bad).or_else(|| {
                    objs.iter()
                        .copied()
                        .find(|o| !(ability(*o) || feel_bad && owns_target(game, seat, *o)))
                        .map(Target::Obj)
                })
            };
            if let Some(t) = auto {
                chosen.push(t);
                selected = true;
                continue;
            }
        }
        // `HumanPlayer.chooseTarget`: not required (the client offers "done") when there's nothing to choose or the
        // fewest targets are chosen
        let nothing = players.is_empty() && objs.is_empty();
        let required = !cancellable && !nothing && chosen.len() < min;
        let q = Query::Target { players: players.clone(), objs: objs.clone(), required };
        match ask(game, d, seat, &q) {
            Answer::Player(s) if players.contains(&s) => {
                chosen.push(Target::Player(s));
                selected = true;
                added_in_prompt = true;
            }
            Answer::Obj(o) if objs.contains(&o) || clicks_itself(game, def, src, o) => {
                chosen.push(Target::Obj(o));
                selected = true;
                added_in_prompt = true;
            }
            Answer::Player(s) if chosen.contains(&Target::Player(s)) => chosen.retain(|t| *t != Target::Player(s)),
            Answer::Obj(o) if chosen.contains(&Target::Obj(o)) => chosen.retain(|t| *t != Target::Obj(o)),
            // `TargetImpl.chooseTarget`: a prompt that added a target and ended with "done" is followed by one more
            // (its loop goes round again when the count changed)
            Answer::Bool(_) if chosen.len() >= min && divided => return Some(chosen),
            Answer::Bool(_) if chosen.len() >= min && added_in_prompt => added_in_prompt = false,
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
        Zone::Battlefield => o.controller == seat,
        // a spell: whoever cast it (a card cast from an opponent's exile, Ragavan's)
        Zone::Stack => game
            .stack
            .iter()
            .find(|i| i.card == id && i.ability.is_none())
            .map_or(o.controller == seat, |i| i.controller == seat),
        _ => o.owner == seat,
    }
}

/// `src`: the spell or ability's source, for its filters ("another")
pub fn possible_targets(game: &Game, seat: Seat, def: &TargetDef, src: Option<ObjId>) -> (Vec<Seat>, Vec<ObjId>) {
    // a player with hexproof can't be the target of their opponents' spells and abilities
    let players: Vec<Seat> = (0..game.seats())
        .filter(|s| !game.player(*s).lost && (*s == seat || !game.player_has(*s, &AbilityDef::HexproofAbility)))
        .collect();
    let permanents = |f: &dyn Fn(ObjId) -> bool| -> Vec<ObjId> {
        game.battlefield.iter().copied().filter(|id| f(*id) && targetable(game, seat, *id, src)).collect()
    };
    match def {
        TargetDef::TargetAnyTarget { another, .. } => (
            players.into_iter().filter(|p| !game.chosen_before.contains(&Target::Player(*p))).collect(),
            permanents(&|id| {
                [CardType::Creature, CardType::Planeswalker, CardType::Battle].iter().any(|t| game.has_type(id, *t))
                    && !(*another && Some(id) == src)
                    && !game.chosen_before.contains(&Target::Obj(id))
            }),
        ),
        TargetDef::TargetPlayer => (players, Vec::new()),
        TargetDef::TargetOpponent => (players.into_iter().filter(|s| *s != seat).collect(), Vec::new()),
        TargetDef::TargetPlayerOrPlaneswalker => (players, permanents(&|id| game.has_type(id, CardType::Planeswalker))),
        TargetDef::TargetOpponentOrPlaneswalker => (
            players.into_iter().filter(|s| *s != seat).collect(),
            permanents(&|id| game.has_type(id, CardType::Planeswalker) && game.obj(id).controller != seat),
        ),
        TargetDef::TargetPermanent { filter, .. } => {
            (Vec::new(), permanents(&|id| game.filter_matches(filter, seat, src, id)))
        }
        TargetDef::TargetSpell { filter } => (
            Vec::new(),
            game.stack
                .iter()
                .rev()
                // a spell is an illegal target for itself (rule 114.4)
                .filter(|i| i.ability.is_none() && i.delayed.is_none() && Some(i.card) != src)
                .map(|i| i.card)
                .filter(|c| game.filter_matches(filter, seat, src, *c))
                .collect(),
        ),
        TargetDef::TargetTriggeredAbilityOrColorlessSpell => (
            Vec::new(),
            game.stack
                .iter()
                .rev()
                .filter(|i| {
                    if i.ability.is_none() && i.delayed.is_none() {
                        Some(i.card) != src && game.colors(i.card).is_empty()
                    } else {
                        i.delayed.is_some()
                            || i.ability.is_some_and(|(k, _)| {
                                matches!(game.item_def(i).abilities.get(k), Some(AbilityDef::TriggeredAbility { .. }))
                            })
                    }
                })
                .map(|i| i.card)
                .collect(),
        ),
        TargetDef::TargetStackAbility { .. } => (
            Vec::new(),
            game.stack
                .iter()
                .rev()
                // (`TargetActivatedOrTriggeredAbility` lets a trigger target itself, as Java does)
                .filter(|i| i.ability.is_some() || i.delayed.is_some())
                .map(|i| i.card)
                .collect(),
        ),
        TargetDef::TargetSpellOrPermanent { filter } => {
            let mut objs: Vec<ObjId> = game
                .stack
                .iter()
                .rev()
                .filter(|i| i.ability.is_none() && i.delayed.is_none() && Some(i.card) != src)
                .map(|i| i.card)
                .collect();
            objs.extend(permanents(&|id| game.filter_matches(filter, seat, src, id)));
            (Vec::new(), objs)
        }
        TargetDef::TargetCardInGraveyard { filter, .. } => (
            Vec::new(),
            game.players
                .iter()
                .flat_map(|p| p.graveyard.iter().copied())
                .filter(|c| game.filter_matches(filter, seat, src, *c))
                .collect(),
        ),
    }
}

/// Whether `seat` may target a permanent with a spell or ability of `src`: not an opponent's with hexproof, nor one
/// with protection from a color of `src`.
fn targetable(game: &Game, seat: Seat, id: ObjId, src: Option<ObjId>) -> bool {
    (game.obj(id).controller == seat
        || !game.has_ability(id, &AbilityDef::HexproofAbility)
        || (game.has_type(id, CardType::Creature) && nowhere_to_run(game, game.obj(id).controller)))
        && !src.is_some_and(|s| game.protected_from(id, s))
}

/// Nowhere to Run: an opponent of `seat` controls one, so `seat`'s creatures can be targeted as though they didn't
/// have hexproof, and their ward abilities don't trigger
fn nowhere_to_run(game: &Game, seat: Seat) -> bool {
    game.battlefield.iter().any(|b| {
        let o = game.obj(*b);
        o.controller != seat
            && o.def.abilities.iter().any(|a| match a {
                AbilityDef::SimpleStaticAbility { effects, .. } => {
                    effects.iter().any(|e| matches!(e, EffectDef::NowhereToRunHexproofEffect))
                }
                _ => false,
            })
    })
}

/// A player's energy counters
fn energy(game: &Game, seat: Seat) -> i32 {
    game.player(seat).counters.iter().filter(|(n, _)| n == "energy").map(|(_, k)| *k).sum()
}

/// Gives a player energy counters, or takes them (a negative amount)
fn add_energy(game: &mut Game, seat: Seat, n: i32) {
    let counters = &mut game.player_mut(seat).counters;
    match counters.iter_mut().find(|(c, _)| c == "energy") {
        Some((_, k)) => *k += n,
        None => counters.push(("energy".to_string(), n)),
    }
    counters.retain(|(_, k)| *k != 0);
}

/// A player's poison counters (`CounterType.POISON`)
pub fn poison(game: &Game, seat: Seat) -> i32 {
    game.player(seat).counters.iter().filter(|(n, _)| n == "poison").map(|(_, k)| *k).sum()
}

/// Gives a player poison counters (infect or toxic damage to a player, `Player.addCounters`); 704.5c makes them lose
/// at ten
pub fn add_poison(game: &mut Game, seat: Seat, n: i32) {
    if n <= 0 {
        return;
    }
    let counters = &mut game.player_mut(seat).counters;
    match counters.iter_mut().find(|(c, _)| c == "poison") {
        Some((_, k)) => *k += n,
        None => counters.push(("poison".to_string(), n)),
    }
}

/// The controller scries `amount`: the cards to put on the bottom, then the rest on top. False: the decider stopped.
fn scry(game: &mut Game, d: &mut dyn Decider, seat: Seat, amount: i32, rule: &str) -> bool {
    let cards: Vec<ObjId> = game.player(seat).library.iter().take(amount.max(0) as usize).copied().collect();
    if cards.is_empty() {
        return true;
    }
    let Some(bottom) = choose_cards(game, d, seat, &cards, &cards, 0, cards.len(), rule) else {
        return false;
    };
    let top: Vec<ObjId> = cards.iter().copied().filter(|c| !bottom.contains(c)).collect();
    put_on_library(game, d, seat, &bottom, false, true) && put_on_library(game, d, seat, &top, true, true)
}

/// How many objects a target takes: at least, at most.
fn target_counts(def: &TargetDef) -> (usize, usize) {
    match def {
        TargetDef::TargetPermanent { min, max, .. }
        | TargetDef::TargetCardInGraveyard { min, max, .. }
        | TargetDef::TargetStackAbility { min, max }
        | TargetDef::TargetAnyTarget { min, max, .. } => (*min, *max),
        _ => (1, 1),
    }
}

/// Whether a target can be chosen at all (an "up to" target always can).
fn target_possible(game: &Game, seat: Seat, def: &TargetDef, src: Option<ObjId>) -> bool {
    // `TargetCreaturePermanentAmount(DynamicValue, …)`: no fewest number, so it never stops a spell being cast
    if matches!(def, TargetDef::TargetPermanent { distribute: Some(crate::card::Amount::Of(_)), .. }) {
        return true;
    }
    let (players, objs) = possible_targets(game, seat, def, src);
    target_counts(def).0 == 0 || !players.is_empty() || !objs.is_empty()
}

fn target_zcc(game: &Game, t: Target) -> u32 {
    match t {
        Target::Obj(o) => game.obj(o).zcc,
        Target::Player(_) => 0,
    }
}

/// `TargetSpellOrPermanent.canTarget` compares the card's id with the spell's, so a spell of it can be clicked as its
/// own target (it isn't offered) and stays legal (Fatehold Charm returning itself to hand)
fn clicks_itself(game: &Game, def: &TargetDef, src: Option<ObjId>, o: ObjId) -> bool {
    matches!(def, TargetDef::TargetSpellOrPermanent { .. }) && src == Some(o) && game.obj(o).zone == Zone::Stack
}

fn target_legal(game: &Game, seat: Seat, def: &TargetDef, src: Option<ObjId>, t: Target, zcc: u32) -> bool {
    if let Target::Obj(o) = t {
        if clicks_itself(game, def, src, o) {
            return game.obj(o).zcc == zcc;
        }
    }
    let (players, objs) = possible_targets(game, seat, def, src);
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
    let paid = pay_mana_from(game, d, seat, cost);
    // what's left of the stock: mana spent from it (or gone) isn't there for a later payment
    let p = game.player_mut(seat);
    for k in 0..6 {
        *kind(&mut p.mana_stock, k) = (*kind(&mut p.mana_stock, k)).min(*kind(&mut p.mana_pool, k));
    }
    paid
}

/// `handlePhyrexianManaCosts`: each phyrexian symbol, 2 life instead if the player wants; the symbols leave the cost
/// and the life to pay is returned (none when the player stopped)
fn ask_phyrexian(game: &mut Game, d: &mut dyn Decider, seat: Seat, cost: &mut ManaCost) -> Option<i32> {
    let mut life = 0;
    for (k, l) in ["W", "U", "B", "R", "G"].iter().enumerate() {
        for _ in 0..cost.phyrexian[k] {
            if game.player(seat).life < 2 {
                continue;
            }
            let q = Query::YesNo { message: format!("Pay 2 life instead of {{{}}}?", l) };
            let yes = loop {
                match ask(game, d, seat, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return None,
                    _ => {}
                }
            };
            if yes {
                cost.colored[k] -= 1;
                life += 2;
            }
        }
        cost.phyrexian[k] = 0;
    }
    Some(life)
}

fn pay_mana_from(game: &mut Game, d: &mut dyn Decider, seat: Seat, cost: &ManaCost) -> bool {
    let mut unpaid = cost.clone();
    let Some(life) = ask_phyrexian(game, d, seat, &mut unpaid) else { return false };
    if life > 0 {
        lose_life(game, seat, life);
    }
    let mut stock = Mana::default();
    {
        let p = game.player_mut(seat);
        for k in 0..6 {
            *kind(&mut stock, k) = (*kind(&mut p.mana_stock, k)).min(*kind(&mut p.mana_pool, k));
        }
    }
    let mut unlocked = Mana::default();
    let mut fresh_conditional = 0;
    let mut special_used = false;
    loop {
        pay_from_pool(game, seat, &mut unpaid, &mut stock, &mut unlocked, &mut fresh_conditional);
        game.player_mut(seat).mana_stock = stock;
        if unpaid.mana_value() == 0 {
            return true;
        }
        let sources: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                let o = game.obj(*id);
                o.controller == seat
                    && o.def.abilities.iter().any(|a| a.mana().is_some() && (!o.tapped || !a.taps_source()))
            })
            .collect();
        // after a special payment the spell's mana abilities can't be activated (`ActivationManaAbilityStep.AFTER`)
        let sources = if special_used { Vec::new() } else { sources };
        let specials = special_mana_actions(game, seat, &unpaid);
        let q = Query::PlayMana { cost: unpaid_text(&unpaid), sources };
        game.special_mana = !specials.is_empty();
        let answer = ask(game, d, seat, &q);
        game.special_mana = false;
        match answer {
            Answer::Text(t) if t == "special" && !specials.is_empty() => {
                match special_mana_payment(game, d, seat, &specials, &unpaid) {
                    Some(true) => special_used = true,
                    Some(false) => {}
                    None => return false,
                }
            }
            Answer::Obj(_) if special_used => {}
            Answer::Obj(id) => {
                // a payment offers a source's conditional mana too, even what it can't spend
                let paying = game.paying_for.get();
                game.paying_for.set(Some(ANY_SPELL));
                let offered = mana_sources_with_kinds(game, seat).into_iter().find(|(s, _)| *s == id);
                game.paying_for.set(paying);
                if let Some((_, abilities)) = offered {
                    let choices = auto_pay_abilities(&abilities, &unpaid);
                    let chosen = if choices.len() == 1 {
                        Some(choices[0])
                    } else {
                        // CHOOSE_ABILITY: an ability of that permanent, or nothing
                        let q = Query::ChooseAbility {
                            source: id,
                            abilities: choices.iter().map(|(i, _, _)| *i).collect(),
                        };
                        match ask(game, d, seat, &q) {
                            Answer::Ability(o, i) if o == id => choices.iter().find(|(j, _, _)| *j == i).copied(),
                            Answer::Stop => return false,
                            _ => None,
                        }
                    };
                    if let Some((i, m, k)) = chosen {
                        let spendable = mana_sources_with_kinds(game, seat)
                            .iter()
                            .any(|(s, a)| *s == id && a.iter().any(|(j, _, _)| *j == i));
                        if !spendable {
                            // tapped for mana this payment can't use: it stays in the pool for one that can (its
                            // life paid too, and its mana cost)
                            tap(game, id);
                            match pay_mana_ability_cost(game, d, seat, id, i) {
                                Some(true) => {}
                                Some(false) => {
                                    untap_undo(game, id);
                                    continue;
                                }
                                None => return false,
                            }
                            for c in game.obj(id).def.abilities[i].mana_costs().to_vec() {
                                if let CostDef::PayLifeCost { amount } = c {
                                    lose_life(game, seat, amount);
                                }
                                if let CostDef::PayEnergyCost { amount } = c {
                                    add_energy(game, seat, -amount);
                                }
                                if let CostDef::RemoveCountersSourceCost { counter } = c {
                                    game.obj_mut(id).add_counters(&counter.name, -counter.amount);
                                }
                            }
                            mana_spent_riders(game, id, i, None);
                            // of any color: `ManaUtil.tryToAutoSelectAManaColor` picks it, as the mana is made
                            let mut m = m;
                            if matches!(
                                game.obj(id).def.abilities[i],
                                AbilityDef::ConditionalManaAbility { color: None, colorless: false, .. }
                                    | AbilityDef::ActivateIfConditionAnyColorManaAbility { .. }
                            ) {
                                let n = match &game.obj(id).def.abilities[i] {
                                    AbilityDef::ConditionalManaAbility { amount, .. } => *amount,
                                    _ => 1,
                                };
                                m = match mana_combination(game, d, seat, id, i) {
                                    Some(Some(m)) => m,
                                    Some(None) => return false,
                                    None => {
                                        let Some(c) = any_color(game, d, seat, &unpaid, n) else { return false };
                                        Mana::of(c, n)
                                    }
                                };
                            }
                            game.player_mut(seat).conditional_pool.push((m, id, i));
                            fresh_conditional += 1;
                            continue;
                        }
                        // `HumanPlayer.activateAbility` sets the stock: what's in the pool now isn't fresh
                        stock = game.player(seat).mana_pool;
                        game.player_mut(seat).mana_stock = stock;
                        let before = game.player(seat).mana_pool;
                        let conditional_before = game.player(seat).conditional_pool.len();
                        if !activate_mana(game, d, seat, id, i, m, k, &unpaid) {
                            return false;
                        }
                        // the sources that paid its own mana cost set the stock again, each as it was activated
                        // (Transmogrant Altar's {B}: a Mountain tapped for it leaves its {R} in the stock)
                        {
                            let p = game.player_mut(seat);
                            for k in 0..6 {
                                *kind(&mut stock, k) = (*kind(&mut p.mana_stock, k)).min(*kind(&mut p.mana_pool, k));
                            }
                        }
                        // what paying its own cost left of conditional mana is fresh too (Nykthos paid with Sunken
                        // Citadel)
                        fresh_conditional +=
                            game.player(seat).conditional_pool.len().saturating_sub(conditional_before) as u32;
                        // conditional mana keeps its condition: what this payment doesn't spend of it waits in the
                        // pool for what it can pay for (Sunken Citadel's, for land abilities)
                        if matches!(game.obj(id).def.abilities.get(i), Some(AbilityDef::ConditionalManaAbility { .. }))
                        {
                            let p = game.player_mut(seat);
                            let mut made = Mana::default();
                            for c in 0..5 {
                                made.colored[c] = p.mana_pool.colored[c].saturating_sub(before.colored[c]);
                            }
                            made.colorless = p.mana_pool.colorless.saturating_sub(before.colorless);
                            p.mana_pool = before;
                            p.conditional_pool.push((made, id, i));
                            fresh_conditional += 1;
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

/// The spell being paid for, if a spell is (not an ability, not a playable check)
fn spell_paid_for(game: &Game) -> Option<ObjId> {
    let id = game.paying_for.get().filter(|id| *id != ANY_SPELL)?;
    (!game.paying_ability.get() && game.obj(id).zone == Zone::Stack).then_some(id)
}

/// `AlternateManaPaymentAbility.addSpecialAction`: the special actions that can pay part of what's unpaid of the
/// spell being paid for, as the indices of its delve, convoke and improvise abilities
fn special_mana_actions(game: &Game, seat: Seat, unpaid: &ManaCost) -> Vec<usize> {
    let Some(spell) = spell_paid_for(game) else { return Vec::new() };
    let def = &game.obj(spell).def;
    let mut out = Vec::new();
    for (i, a) in def.abilities.iter().enumerate() {
        let usable = match a {
            AbilityDef::DelveAbility => unpaid.generic > 0 && !game.player(seat).graveyard.is_empty(),
            AbilityDef::ImproviseAbility => unpaid.generic > 0 && !improvise_artifacts(game, seat).is_empty(),
            AbilityDef::ConvokeAbility => !convoke_creatures(game, seat, unpaid).is_empty(),
            _ => false,
        };
        if usable {
            out.push(i);
        }
    }
    out
}

fn improvise_artifacts(game: &Game, seat: Seat) -> Vec<ObjId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|c| game.obj(*c).controller == seat && !game.obj(*c).tapped && game.has_type(*c, CardType::Artifact))
        .collect()
}

/// The untapped creatures that can convoke: any while generic mana is unpaid, otherwise those of an unpaid color
fn convoke_creatures(game: &Game, seat: Seat, unpaid: &ManaCost) -> Vec<ObjId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.controller == seat
                && !o.tapped
                && game.has_type(*c, CardType::Creature)
                && (unpaid.generic > 0 || game.colors(*c).iter().any(|col| unpaid.colored[col.index()] > 0))
        })
        .collect()
}

/// Picks up to `max` of `objs` (PICK_TARGET each, `HumanPlayer.chooseTarget`): a pick again takes it back; done once
/// at least `min` are picked. With exactly `min` to pick and `min` required, they're picked for the player
/// (`tryToAutoChoose`). None: stopped; an empty pick with `min` above 0 is a cancel.
fn pick_objects(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    objs: &[ObjId],
    min: usize,
    max: usize,
) -> Option<Vec<ObjId>> {
    if min > 0 && objs.len() == min {
        return Some(objs.to_vec());
    }
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let required = chosen.len() < min;
        let q = Query::Target { players: Vec::new(), objs: objs.to_vec(), required };
        match ask(game, d, seat, &q) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if objs.contains(&o) => {
                chosen.push(o);
                if chosen.len() >= max {
                    return Some(chosen);
                }
            }
            Answer::Stop => return None,
            Answer::Bool(_) if chosen.len() >= min => return Some(chosen),
            Answer::Bool(_) => return Some(Vec::new()),
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}

/// `HumanPlayer.activateSpecialAction` while paying mana: the special actions to choose from (CHOOSE_ABILITY, asked
/// even for one), then the chosen one: delve exiles cards from the graveyard (`ExileFromGraveCost`, up to the generic
/// mana unpaid), improvise taps artifacts (up to the generic mana unpaid), convoke taps one creature, its color asked
/// when it could pay more than one kind. Each adds its mana to the pool, spent at once. Some(true): something was
/// paid; None: stopped.
fn special_mana_payment(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    specials: &[usize],
    unpaid: &ManaCost,
) -> Option<bool> {
    let Some(spell) = spell_paid_for(game) else { return Some(false) };
    let q = Query::ChooseAbility { source: spell, abilities: specials.to_vec() };
    let k = match ask(game, d, seat, &q) {
        Answer::Ability(o, i) if o == spell && specials.contains(&i) => i,
        Answer::Stop => return None,
        _ => return Some(false),
    };
    let add = |game: &mut Game, m: Mana| {
        let p = game.player_mut(seat);
        for c in 0..5 {
            p.mana_pool.colored[c] += m.colored[c];
        }
        p.mana_pool.colorless += m.colorless;
    };
    match game.obj(spell).def.abilities[k] {
        AbilityDef::DelveAbility => {
            let cards: Vec<ObjId> = game.player(seat).graveyard.clone();
            let max = cards.len().min(unpaid.generic as usize);
            let chosen = pick_objects(game, d, seat, &cards, 0, max)?;
            for c in &chosen {
                game.move_to(*c, Zone::Exile);
            }
            add(game, Mana { colorless: chosen.len() as u32, ..Mana::default() });
            Some(!chosen.is_empty())
        }
        AbilityDef::ImproviseAbility => {
            let artifacts = improvise_artifacts(game, seat);
            let chosen = pick_objects(game, d, seat, &artifacts, 1, unpaid.generic as usize)?;
            for c in &chosen {
                tap(game, *c);
                if log::kept(game) {
                    let line = format!("Improvise: {} taps {} to pay {{1}}", log::player(seat), log::object(game, *c));
                    log::inform(game, line);
                }
                add(game, Mana { colorless: 1, ..Mana::default() });
            }
            Some(!chosen.is_empty())
        }
        AbilityDef::ConvokeAbility => {
            let creatures = convoke_creatures(game, seat, unpaid);
            let chosen = pick_objects(game, d, seat, &creatures, 1, 1)?;
            let Some(c) = chosen.first().copied() else { return Some(false) };
            tap(game, c);
            // `ConvokeEffect.buildChoice`: its colors that are unpaid, and Colorless beside more than one
            let names = ["White", "Blue", "Black", "Red", "Green"];
            let mut choices: Vec<String> = Vec::new();
            for col in [Color::B, Color::U, Color::G, Color::R, Color::W] {
                if game.colors(c).contains(&col) && unpaid.colored[col.index()] > 0 {
                    choices.push(names[col.index()].to_string());
                }
            }
            let pick = match choices.len() {
                0 => "Colorless".to_string(),
                1 => choices[0].clone(),
                _ => {
                    choices.push("Colorless".into());
                    let message = format!("Choose mana color to reduce from {}", game.obj(c).name());
                    let q = Query::Choice { message, choices: choices.clone() };
                    loop {
                        match ask(game, d, seat, &q) {
                            Answer::Text(t) if choices.contains(&t) => break t,
                            Answer::Stop => return None,
                            _ => {}
                        }
                        if d.stopped() {
                            return None;
                        }
                    }
                }
            };
            let m = match names.iter().position(|n| *n == pick) {
                Some(i) => Mana::of(Color::ALL[i], 1),
                None => Mana { colorless: 1, ..Mana::default() },
            };
            add(game, m);
            if log::kept(game) {
                let line = format!(
                    "Convoke: {} taps {} to pay one {} mana",
                    log::player(seat),
                    log::object(game, c),
                    pick.to_lowercase()
                );
                log::inform(game, line);
            }
            Some(true)
        }
        _ => Some(false),
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
    m: Mana,
    k: ManaKind,
    unpaid: &ManaCost,
) -> bool {
    let made = activate_mana_paying(game, d, seat, id, i, m, k, unpaid);
    // `processManaAction` once it's resolved
    game.short_lki.clear();
    game.mana_batch += 1;
    made
}

#[allow(clippy::too_many_arguments)]
fn activate_mana_paying(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    id: ObjId,
    i: usize,
    mut m: Mana,
    k: ManaKind,
    unpaid: &ManaCost,
) -> bool {
    let taps = game.obj(id).def.abilities[i].taps_source();
    // a mana cost cancelled undoes the whole activation (`PlayerImpl.playManaAbility` restores its bookmark)
    let before_costs = game.obj(id).def.abilities[i].mana_ability_mana_cost().map(|_| game.clone());
    // Heritage Druid: what it makes once (its playable amount counts every three it could tap)
    if let AbilityDef::ColoredManaAbility { color, amount, no_tap: true, .. } = &game.obj(id).def.abilities[i] {
        m = Mana::of(*color, *amount);
    }
    if taps {
        tap(game, id);
    }
    // "tap an untapped permanent you control": asked, unless only enough are left
    for c in game.obj(id).def.abilities[i].mana_costs().to_vec() {
        if let CostDef::TapTargetCost { filter, amount } = c.clone() {
            let possible = if taps {
                mana_tap_targets(game, seat, id, &filter)
            } else {
                // the source can be one of them (Heritage Druid)
                matching(game, &filter, seat, id)
                    .into_iter()
                    .filter(|o| game.obj(*o).controller == seat && !game.obj(*o).tapped)
                    .collect()
            };
            // `TapTargetCost`: its target isn't required (done with none chosen cancels the ability)
            let Some(chosen) = choose_cards_with(game, d, seat, &possible, &possible, amount, amount, "tap", true)
            else {
                return false;
            };
            if chosen.len() < amount {
                // the cost wasn't paid: the ability isn't activated
                if taps {
                    untap_undo(game, id);
                }
                return true;
            }
            for o in chosen {
                tap(game, o);
            }
        }
        // "Sacrifice a Goblin" (Skirk Prospector): `SacrificeTargetCost`, as for any ability
        if let CostDef::SacrificeTargetCost { .. } = c {
            if !pay_cost(game, d, seat, id, &c) {
                if taps {
                    untap_undo(game, id);
                }
                return !d.stopped();
            }
        }
    }
    if let AbilityDef::ConditionalManaAbility { cant_counter: true, .. } = game.obj(id).def.abilities[i] {
        if let Some(s) = game.paying_for.get().filter(|_| !game.paying_ability.get()) {
            game.obj_mut(s).uncounterable = true;
        }
    }
    // `AbilityImpl.activate`: a mana ability pays its other costs before its mana cost (Vessel of Volatility is
    // sacrificed before its {1}{R} is paid)
    for c in game.obj(id).def.abilities[i].mana_costs().to_vec() {
        match c {
            CostDef::SacrificeSourceCost => sacrifice(game, id),
            CostDef::PayLifeCost { amount } => lose_life(game, seat, amount),
            CostDef::PayEnergyCost { amount } => add_energy(game, seat, -amount),
            CostDef::RemoveCountersSourceCost { counter } => {
                game.obj_mut(id).add_counters(&counter.name, -counter.amount)
            }
            _ => {}
        }
    }
    let batch = game.mana_batch;
    match pay_mana_ability_cost(game, d, seat, id, i) {
        Some(true) => {}
        Some(false) => {
            match before_costs {
                Some(b) => *game = b,
                None => untap_undo(game, id),
            }
            return true;
        }
        None => return false,
    }
    // a source clicked to pay its own mana cost: `HumanPlayer.playManaHandling` clears `currentlyUnpaidMana` after
    // that click, so the color isn't autoselected (Giant's Boulder's {1})
    let owed_all = ManaCost { colored: [1; 5], ..ManaCost::default() };
    let choosing = if game.mana_batch != batch { &owed_all } else { unpaid };
    if let AbilityDef::NykthosShrineToNyxManaAbility { cost, .. } = game.obj(id).def.abilities[i].clone() {
        // its {2} first, then the color (`NykthosDynamicManaEffect`, never chosen for the player). Cancelled, the
        // activation is undone (`restoreState`): what was tapped for it untaps
        let before = game.clone();
        for c in &cost {
            if let CostDef::ManaCost { cost } = c {
                let Ok(c) = ManaCost::parse(cost) else { return false };
                // paid for Nykthos's own ability: a land's (Sunken Citadel's mana can go to it)
                let paying = (game.paying_for.get(), game.paying_ability.get());
                game.paying_for.set(Some(id));
                game.paying_ability.set(true);
                let paid = pay_mana(game, d, seat, &c);
                game.paying_for.set(paying.0);
                game.paying_ability.set(paying.1);
                if !paid {
                    *game = before;
                    game.obj_mut(id).tapped = false;
                    return !d.stopped();
                }
            }
        }
        let names = ["White", "Blue", "Black", "Red", "Green"];
        let choices: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        let q = Query::Choice { message: "Choose a color for devotion of Nykthos".into(), choices };
        let c = loop {
            match ask(game, d, seat, &q) {
                Answer::Text(t) => {
                    if let Some(k) = names.iter().position(|n| *n == t) {
                        break Color::ALL[k];
                    }
                }
                Answer::Stop => return false,
                _ => {}
            }
        };
        m = Mana::of(c, devotion(game, seat, c));
    }
    // Mox Amber: one of the colors among them (`AnyColorPermanentTypesManaEffect.produceMana`); none, no mana
    if let AbilityDef::AnyColorPermanentTypesManaAbility { filter, .. } = game.obj(id).def.abilities[i].clone() {
        let allowed = colors_among(game, seat, id, &filter);
        m = Mana::default();
        if !allowed.is_empty() {
            let Some(c) = color_among(game, d, seat, choosing, &allowed) else { return false };
            m = Mana::of(c, 1);
        }
    }
    let conditional_any = matches!(
        game.obj(id).def.abilities[i],
        AbilityDef::ConditionalManaAbility { color: None, colorless: false, .. }
            | AbilityDef::ActivateIfConditionAnyColorManaAbility { .. }
    );
    if k == ManaKind::AnyColor
        || conditional_any
        || matches!(game.obj(id).def.abilities[i], AbilityDef::AnyColorManaAbility { .. })
    {
        let n = match &game.obj(id).def.abilities[i] {
            AbilityDef::AnyColorManaAbility { amount, .. } | AbilityDef::ConditionalManaAbility { amount, .. } => {
                *amount
            }
            _ => 1,
        };
        m = match mana_combination(game, d, seat, id, i) {
            Some(Some(m)) => m,
            Some(None) => return false,
            None => {
                let Some(c) = any_color(game, d, seat, choosing, n) else { return false };
                Mana::of(c, n)
            }
        };
    }
    mana_spent_riders(game, id, i, Some(unpaid));
    let pool = &mut game.player_mut(seat).mana_pool;
    for c in Color::ALL {
        pool.colored[c.index()] += m.colored[c.index()];
    }
    pool.colorless += m.colorless;
    let extra = tap_for_mana_extra(game, seat, id);
    let made = m.total() as i32 + extra.len() as i32;
    for k in extra {
        game.player_mut(seat).mana_pool.colored[k] += 1;
    }
    note(game, EventKind::Mana, Some(seat), Some(id), None, made);
    // a mana ability's other effects happen at once (a pain land's damage)
    let effects = game.obj(id).def.abilities[i].mana_effects().to_vec();
    if !effects.is_empty() {
        let zcc = game.obj(id).zcc;
        apply_effects(game, d, &StackItem::spell(id, seat), &effects, &[], (id, zcc));
    }
    true
}

/// A mana ability's mana cost (Castle Garenbrig's {2}{G}{G}), paid as for a land's ability. Some(false): not paid,
/// and what paying it did is undone; None: stopped.
/// A mana ability's exert cost, and what its mana does for the spell it pays for (Arena of Glory: a creature spell
/// gains haste until end of turn, on the permanent it becomes)
fn mana_spent_riders(game: &mut Game, id: ObjId, i: usize, unpaid: Option<&ManaCost>) {
    if game.obj(id).def.abilities[i].mana_costs().contains(&CostDef::ExertSourceCost) {
        game.obj_mut(id).exerted = true;
    }
    // (the mana is spent on the spell only if what's unpaid can take it: `ManaSpentDelayedTriggeredAbility`)
    if let AbilityDef::ColoredManaAbility { creature_spell_haste: true, color, .. } = game.obj(id).def.abilities[i] {
        if !unpaid.is_some_and(|u| u.generic > 0 || u.colored[color.index()] > 0) {
            return;
        }
        if let Some(s) = game.paying_for.get().filter(|_| !game.paying_ability.get()) {
            if s != ANY_SPELL && game.obj(s).zone == Zone::Stack && game.has_type(s, CardType::Creature) {
                let zcc = game.obj(s).zcc + 1;
                Arc::make_mut(&mut game.grants).push(Grant {
                    target: s,
                    zcc,
                    ability: AbilityDef::HasteAbility,
                    until_end_of_turn: true,
                    until_turn_of: None,
                });
            }
        }
    }
}

fn pay_mana_ability_cost(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId, i: usize) -> Option<bool> {
    let Some(c) = game.obj(id).def.abilities[i].mana_ability_mana_cost().map(str::to_string) else {
        return Some(true);
    };
    let before = game.clone();
    let c = ManaCost::parse(&c).ok()?;
    let paying = (game.paying_for.get(), game.paying_ability.get());
    game.paying_for.set(Some(id));
    game.paying_ability.set(true);
    let paid = pay_mana(game, d, seat, &c);
    game.paying_for.set(paying.0);
    game.paying_ability.set(paying.1);
    if paid {
        return Some(true);
    }
    if d.stopped() {
        return None;
    }
    *game = before;
    Some(false)
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
    if game.has_type(id, CardType::Battle) {
        lethal = lethal.min(game.obj(id).counter("defense"));
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
/// unlocked stock. `ManaPool.pay`: conditional mana the spell can spend goes first, but (stock or not) only while some
/// mana is fresh, so it can leave fresh mana unspent; `fresh_conditional` counts conditional mana made this payment.
fn pay_from_pool(
    game: &mut Game,
    seat: Seat,
    unpaid: &mut ManaCost,
    stock: &mut Mana,
    unlocked: &mut Mana,
    fresh_conditional: &mut u32,
) {
    let mut pool = game.player(seat).mana_pool;
    let (manual, unrestricted) = game.pool_mode(seat);
    if unrestricted {
        // (all of the pool pays, what was there before too)
        *stock = Mana::default();
    }
    let mut conditional = std::mem::take(&mut game.player_mut(seat).conditional_pool);
    let mut tally = [0u32; 6];
    let mut spend = |pool: &mut Mana, stock: &mut Mana, unlocked: &mut Mana, k: usize, want: u32| -> u32 {
        let mut paid = 0;
        while paid < want {
            // (`ManaPool.pay`: with automatic payment off, only an unlocked type pays)
            let fresh = !manual && ((0..6).any(|k| *kind(pool, k) > *kind(stock, k)) || *fresh_conditional > 0);
            let usable = fresh
                .then(|| {
                    conditional.iter().position(|(m, src, j)| {
                        (if k < 5 { m.colored[k] } else { m.colorless }) > 0 && conditional_usable(game, seat, *src, *j)
                    })
                })
                .flatten();
            if let Some(c) = usable {
                // one mana of it (Sunken Citadel's two are one entry; one of any color is all of them)
                let m = &mut conditional[c].0;
                let colors = m.colored.iter().filter(|n| **n > 0).count();
                if k < 5 {
                    m.colored[k] -= 1;
                } else {
                    m.colorless -= 1;
                }
                if colors > 1 || m.colored.iter().sum::<u32>() + m.colorless == 0 {
                    // the fresh entries are the last ones (made this payment): spending older conditional mana
                    // leaves the fresh mana there (`wasManaAddedBeyondStock`: Vodalian Arcanist's {C} stays)
                    if c + *fresh_conditional as usize >= conditional.len() {
                        *fresh_conditional = fresh_conditional.saturating_sub(1);
                    }
                    conditional.remove(c);
                }
            } else if fresh && *kind(pool, k) > *kind(stock, k) {
                *kind(pool, k) -= 1;
            } else if *kind(unlocked, k) > 0 && *kind(pool, k) > 0 {
                *kind(unlocked, k) -= 1;
                *kind(stock, k) = kind(stock, k).saturating_sub(1);
                *kind(pool, k) -= 1;
            } else {
                break;
            }
            paid += 1;
            tally[k] += 1;
        }
        paid
    };
    for (k, need) in unpaid.colored.iter_mut().enumerate() {
        *need -= spend(&mut pool, stock, unlocked, k, *need);
    }
    unpaid.colorless -= spend(&mut pool, stock, unlocked, 5, unpaid.colorless);
    // `HybridManaCost.assignPayment`: its first color, else its second
    let mut left = Vec::new();
    for (a, b) in std::mem::take(&mut unpaid.hybrid) {
        if spend(&mut pool, stock, unlocked, a, 1) == 0 && spend(&mut pool, stock, unlocked, b, 1) == 0 {
            left.push((a, b));
        }
    }
    unpaid.hybrid = left;
    // `ManaCostImpl.assignGeneric`: one mana at a time, {C} then {B}{U}{W}{G}{R}, each time from the first that pays
    // (whether the pool has fresh mana, which lets conditional mana pay, can change after each)
    while unpaid.generic > 0 {
        if ![5, 2, 1, 0, 4, 3].into_iter().any(|k| spend(&mut pool, stock, unlocked, k, 1) == 1) {
            break;
        }
        unpaid.generic -= 1;
    }
    for (k, n) in tally.iter().enumerate() {
        game.spent_tally[k] += n;
    }
    let p = game.player_mut(seat);
    p.mana_pool = pool;
    p.conditional_pool = conditional;
}

/// Whether conditional mana made by `src`'s ability `j` can be spent on the spell being paid for.
fn conditional_usable(game: &Game, seat: Seat, src: ObjId, j: usize) -> bool {
    game.obj(src).def.abilities.get(j).is_some_and(|a| match a {
        AbilityDef::ConditionalManaAbility { land_abilities: true, .. } => {
            game.paying_ability.get()
                && game.paying_for.get().is_some_and(|s| s != ANY_SPELL && game.has_type(s, CardType::Land))
        }
        AbilityDef::ConditionalManaAbility { filter, chosen_type, creature_abilities, filter_abilities, .. } => {
            game.paying_for.get().is_some_and(|s| {
                s != ANY_SPELL
                    && *creature_abilities
                    && game.paying_ability.get()
                    && game.has_type(s, CardType::Creature)
                    || s != ANY_SPELL
                        && (*filter_abilities || !game.paying_ability.get())
                        && game.filter_matches(filter, seat, Some(src), s)
                        && (!*chosen_type || game.obj(src).chosen_type.as_ref().is_some_and(|t| game.has_subtype(s, t)))
            })
        }
        _ => false,
    })
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
    for (a, b) in &c.hybrid {
        let l = ["W", "U", "B", "R", "G"];
        s.push_str(&format!("{{{}/{}}}", l[*a], l[*b]));
    }
    for _ in 0..c.colorless {
        s.push_str("{C}");
    }
    s
}

/// Resolves the top of the stack. Java keeps it on the stack while it resolves (`GameImpl.resolve` removes it
/// after), which shows in where things put onto the stack meanwhile are (`Game::resolving`).
fn resolve_top(game: &mut Game, d: &mut dyn Decider) {
    game.short_lki.clear();
    game.resolving = game.stack.last().map(|i| (game.stack.len() - 1, i.card));
    game.resolving_item = game.stack.last().cloned();
    game.resolving_controller = game.stack.last().filter(|i| i.ability.is_none()).map(|i| (i.card, i.controller));
    game.revealed.clear();
    game.looked_at.clear();
    resolve_popped(game, d);
    // `processAction` applies the effects as it resolves: Leyline of the Guildpact's land types end with it
    game.update_guildpact();
    game.revealed.clear();
    game.looked_at.clear();
    game.resolving = None;
    game.resolving_item = None;
    game.resolving_controller = None;
    game.short_lki.clear();
}

fn resolve_popped(game: &mut Game, d: &mut dyn Decider) {
    let item = match Arc::make_mut(&mut game.stack).pop() {
        Some(i) => i,
        None => return,
    };
    let id = item.card;
    let def = game.item_def(&item).clone();
    if let Some(delayed) = &item.delayed {
        for (o, zcc) in &delayed.sacrifice {
            let p = game.obj(*o);
            if p.zone == Zone::Battlefield && p.zcc == *zcc && p.controller == item.controller {
                sacrifice(game, *o);
            }
        }
        // earthbend: `ReturnToBattlefieldUnderOwnerControlTargetEffect`, tapped
        for (o, zcc) in &delayed.return_tapped {
            let c = game.obj(*o);
            if matches!(c.zone, Zone::Graveyard | Zone::Exile) && c.zcc == *zcc {
                let owner = c.owner;
                enter_battlefield(game, d, *o, owner, false);
                game.obj_mut(*o).tapped = true;
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
        // a reflexive trigger with targets: its intervening "if" again, then its targets if they're still legal
        if !delayed.targets.is_empty() {
            if !game.condition_holds(delayed.condition.as_ref(), item.controller, item.card) {
                return;
            }
            if let Some(legal) = legal_targets(game, &item, &delayed.targets) {
                let effects = delayed.effects.clone();
                apply_effects(game, d, &item, &effects, &legal, (item.card, delayed.source_zcc));
            }
            return;
        }
        if !delayed.effects.is_empty() {
            let effects = delayed.effects.clone();
            let fixed: Vec<Target> = delayed
                .fixed
                .iter()
                // a permanent, the spell that set it off (Kumano's "your next creature spell"), or a card that died
                .filter(|(o, z)| {
                    matches!(game.obj(*o).zone, Zone::Battlefield | Zone::Stack | Zone::Graveyard)
                        && game.obj(*o).zcc == *z
                })
                .map(|(o, _)| Target::Obj(*o))
                .collect();
            if delayed.fixed.is_empty() || !fixed.is_empty() {
                let legal = if fixed.is_empty() { Vec::new() } else { vec![fixed] };
                apply_effects(game, d, &item, &effects, &legal, (item.card, delayed.source_zcc));
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
        // a modal triggered ability does its chosen mode
        let modes = def.abilities[index].trigger_modes();
        let (effects, targets) = match item.modes.first() {
            Some(k) if *k < modes.len() => modes[*k],
            _ => (effects, targets),
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
                    game.attach_to(id, to, zcc);
                }
            }
            return;
        }
        // the X it was cast with, for its enters triggers (`GetXValue`)
        game.obj_mut(id).cast_x = item.x;
        enter_battlefield(game, d, id, item.controller, item.kicked);
        return;
    }
    let zcc = game.obj(id).zcc;
    // countered on resolution (608.2b): an adventure then goes to the graveyard, not on its adventure
    let fizzled;
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
        fizzled = fizzles;
        if !fizzles {
            for ((effects, _, part), l) in parts.iter().zip(legal) {
                if let Some(l) = l {
                    apply_effects(game, d, part, effects, &l, (id, zcc));
                }
            }
        }
    } else if !item.spliced.is_empty() {
        // `Spell.resolve`: the spell's ability, then each spliced card's, each with its own targets; with every
        // targeted part's targets illegal the spell doesn't resolve
        let (effects, targets) = def.spell.mode(item.modes.first().copied().unwrap_or(0));
        let mut parts = vec![(effects.to_vec(), targets.to_vec(), item.clone())];
        for (c, ts, zs) in &item.spliced {
            let splice_def = game.obj(*c).def.clone();
            let mut part = item.clone();
            part.targets = ts.clone();
            part.target_zcc = zs.clone();
            parts.push((splice_def.spell.effects.clone(), splice_def.spell.targets.clone(), part));
        }
        let legal: Vec<Option<Vec<Vec<Target>>>> =
            parts.iter().map(|(_, targets, part)| legal_targets(game, part, targets)).collect();
        let targeted = parts.iter().any(|(_, t, _)| !t.is_empty());
        let fizzles = targeted && parts.iter().zip(&legal).all(|((_, t, _), l)| t.is_empty() || l.is_none());
        fizzled = fizzles;
        if !fizzles {
            for ((effects, _, part), l) in parts.iter().zip(legal) {
                if let Some(l) = l {
                    apply_effects(game, d, part, effects, &l, (id, zcc));
                }
            }
        }
    } else {
        let (effects, targets) = def.spell.mode(item.modes.first().copied().unwrap_or(0));
        let legal = legal_targets(game, &item, targets);
        fizzled = legal.is_none();
        if let Some(legal) = legal {
            // `PromiseGiftEffect`, the spell's first effect
            if let Some(to) = item.gift_to {
                give_gift(game, d, &def, to, id);
            }
            apply_effects(game, d, &item, effects, &legal, (id, zcc));
            // an omen: then shuffle this card into its owner's library (`ShuffleIntoLibrarySourceEffect`)
            if def.omen && game.obj(id).zone == Zone::Stack {
                let owner = game.obj(id).owner;
                to_library(game, id, true);
                let mut library = game.player(owner).library.clone();
                game.rng.shuffle(&mut library);
                game.player_mut(owner).library = library;
            }
        }
    }
    if game.obj(id).zone == Zone::Stack {
        // an adventure goes on an adventure: exiled, its creature castable from there
        if game.obj(id).adventurer.as_ref().is_some_and(|f| f.adventure.is_some())
            && !game.obj(id).flashback
            && !fizzled
        {
            game.move_to(id, Zone::Exile);
            game.obj_mut(id).on_adventure = true;
        } else if !fizzled && def.has(&AbilityDef::ReboundAbility) && game.obj(id).cast_from == Some(Zone::Hand) {
            // rebound (`ReboundCastFromHandReplacementEffect`): exiled instead, offered at its owner's next upkeep
            game.move_to(id, Zone::Exile);
            let (zcc, owner) = (game.obj(id).zcc, game.obj(id).owner);
            game.rebound.push((id, zcc, owner));
        } else {
            game.move_to(id, Zone::Graveyard);
            if log::kept(game) && game.obj(id).zone == Zone::Graveyard {
                // `PlayerImpl.moveCardToGraveyardWithInfo`
                let whose =
                    if game.obj(id).owner == item.controller { "their graveyard" } else { "its owner's graveyard" };
                let line = format!(
                    "{} puts {} from stack into {}",
                    log::player(item.controller),
                    log::object(game, id),
                    whose
                );
                log::inform(game, line);
            }
        }
    }
}

/// A Room permanent as its doors make it (`RoomCharacteristicsEffect`): the unlocked halves' names, costs and
/// abilities, after the card's own, then the special action to unlock each locked door
fn room_face(full: &CardDef, doors: [bool; 2]) -> CardDef {
    let mut def = full.clone();
    let unlocked: Vec<&Arc<CardDef>> = full.room.iter().zip(doors).filter(|(_, u)| *u).map(|(h, _)| h).collect();
    def.name = unlocked.iter().map(|h| h.name.as_str()).collect::<Vec<_>>().join(" // ");
    def.cost = unlocked.iter().fold(ManaCost::default(), |c, h| c.plus(&h.cost));
    for h in &unlocked {
        def.abilities.extend(h.abilities.iter().filter(|a| !matches!(a, AbilityDef::SpellAbility)).cloned());
    }
    for (k, h) in full.room.iter().enumerate() {
        if !doors[k] {
            def.abilities.push(AbilityDef::RoomUnlockAbility { left: k == 0, cost: unpaid_text(&h.cost) });
        }
    }
    def
}

fn set_room_face(game: &mut Game, id: ObjId) {
    let Some(full) = game.obj(id).printed_def.clone().filter(|f| !f.room.is_empty()) else { return };
    let doors = game.obj(id).doors;
    // its abilities already waiting or on the stack keep the face they came from
    let old = game.obj(id).def.clone();
    for t in Arc::make_mut(&mut game.stack).iter_mut().chain(game.pending.iter_mut()) {
        if t.card == id && t.ability.is_some() && t.def.is_none() {
            t.def = Some(old.clone());
        }
    }
    game.obj_mut(id).def = Arc::new(room_face(&full, doors));
}

/// `PermanentImpl.unlockDoor`: the door unlocks, and "when you unlock this door" triggers
fn unlock_door(game: &mut Game, id: ObjId, k: usize) {
    if game.obj(id).doors[k] {
        return;
    }
    game.obj_mut(id).doors[k] = true;
    set_room_face(game, id);
    let fired = game.triggers([id], |_, _, _, t| matches!(t, Trigger::DoorUnlocked { left } if *left == (k == 0)));
    game.pending.extend(fired);
}

/// `BecomesCreatureSourceEffect`: the card as the token's kind of creature (its other types gone unless `retain`)
fn becomes_creature(old: &CardDef, token: &crate::card::TokenDef, retain: bool) -> CardDef {
    let mut def = old.clone();
    if !retain {
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
    def
}

/// `setPrepared(true)`: a permanent with a prepare spell, not yet prepared, gets a copy of that spell in exile
fn become_prepared(game: &mut Game, id: ObjId) {
    let o = game.obj(id);
    if o.zone != Zone::Battlefield || o.prepared_copy.is_some() {
        return;
    }
    let Some(spell) = o.def.prepare.clone() else { return };
    let (controller, zcc) = (o.controller, o.zcc);
    let copy = game.create_obj(spell, controller, Zone::Exile);
    Arc::make_mut(&mut game.exile).push(copy);
    let c = game.obj_mut(copy);
    c.token = true;
    c.may_play = Some((controller, u32::MAX));
    c.cast_only = true;
    c.prepared_for = Some((id, zcc));
    game.obj_mut(id).prepared_copy = Some(copy);
}

/// It loses all abilities for as long as `src` stays: a static "it's a creature" of its own (Kaito) stops applying
fn lose_all_abilities(game: &mut Game, id: ObjId, src: ObjId, zcc: u32) {
    let ob = game.obj_mut(id);
    if let Some(def) = ob.becomes_base.take() {
        ob.def = def;
    }
    let old = ob.def.clone();
    let mut def = (*old).clone();
    def.abilities.clear();
    ob.def = Arc::new(def);
    ob.lost_abilities = Some((src, zcc, old.clone()));
    // its abilities already waiting or on the stack are independent of it (113.7a): they keep theirs
    for t in Arc::make_mut(&mut game.stack).iter_mut().chain(game.pending.iter_mut()) {
        if t.card == id && t.ability.is_some() && t.def.is_none() {
            t.def = Some(old.clone());
        }
    }
}

/// 704.5n: an Equipment attached to what isn't a creature (or is gone) becomes unattached
fn unattach_equipment(game: &mut Game) -> bool {
    let mut happened = false;
    let ids: Vec<ObjId> = game.battlefield.iter().copied().collect();
    for id in ids {
        if game.obj(id).attached_to.is_some()
            && game.obj(id).def.has_subtype("Equipment")
            && (!game.attached(id).is_some_and(|to| game.has_type(to, CardType::Creature))
                // an Equipment that's a creature too (Ensoul Artifact) can't equip (704.5n)
                || game.has_type(id, CardType::Creature))
        {
            game.obj_mut(id).attached_to = None;
            happened = true;
        }
    }
    happened
}

/// A static "as long as …, it's a creature" (Kaito, Bane of Nightmares): applied or removed as its condition changes.
/// Java applies continuous effects all the time; here, before each question, trace and state check.
pub(crate) fn refresh_becomes(game: &mut Game) {
    let ids: Vec<ObjId> = game.battlefield.iter().copied().collect();
    for id in ids {
        let o = game.obj(id);
        let base = o.becomes_base.clone().unwrap_or_else(|| o.def.clone());
        // with its abilities lost, the static still makes it a creature (layer 4), but doesn't set its power and
        // toughness (layer 7b, after the loss in layer 6), and gives no abilities
        let lost = o.lost_abilities.as_ref().map(|(_, _, d)| d.clone());
        let found = lost.as_ref().unwrap_or(&base).abilities.iter().find_map(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().find_map(|e| match e {
                EffectDef::ConditionalContinuousEffect { effect, condition } => match &**effect {
                    EffectDef::BecomesCreatureSourceEffect { token, retain, .. } => {
                        Some((token.clone(), *retain, condition.clone()))
                    }
                    _ => None,
                },
                _ => None,
            }),
            _ => None,
        });
        // a Theros god: not a creature while its devotion is too low (layer 4, `LoseCreatureTypeSourceEffect`)
        let devotion = base.abilities.iter().find_map(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().find_map(|e| match e {
                EffectDef::LoseCreatureTypeSourceEffect { amount, less_than } => Some((amount.clone(), *less_than)),
                _ => None,
            }),
            _ => None,
        });
        if let Some((amount, less_than)) = devotion {
            let holds = lost.is_none() && game.value(&amount, o.controller, id, 0) < less_than;
            let applied = o.becomes_base.is_some();
            if holds && !applied {
                let mut def = (*base).clone();
                def.types.retain(|t| *t != CardType::Creature);
                if !def.types.contains(&CardType::Kindred) {
                    def.subtypes.retain(|s| crate::card::NON_CREATURE_TYPES.contains(&s.as_str()));
                    def.all_creature_types = false;
                }
                let o = game.obj_mut(id);
                o.becomes_base = Some(base);
                o.def = Arc::new(def);
                remove_from_combat(game, id);
            } else if !holds && applied {
                let o = game.obj_mut(id);
                o.def = o.becomes_base.take().unwrap();
            }
            continue;
        }
        // Ensoul Artifact: an Aura's "enchanted artifact is a 5/5 creature in addition to its other types"
        let ensoul = game.battlefield.iter().find_map(|a| {
            if game.attached(*a) != Some(id) {
                return None;
            }
            game.obj(*a).def.abilities.iter().find_map(|b| match b {
                AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().find_map(|e| match e {
                    EffectDef::BecomesCreatureAttachedEffect { token } => Some(token.clone()),
                    _ => None,
                }),
                _ => None,
            })
        });
        let (token, retain, holds) = match (found, ensoul) {
            (Some((token, retain, condition)), _) if game.condition_holds(Some(&condition), o.controller, id) => {
                (token, retain, true)
            }
            (_, Some(token)) => (token, true, true),
            (Some((token, retain, _)), None) => (token, retain, false),
            (None, None) if o.becomes_base.is_some() => (crate::card::TokenDef::default(), false, false),
            (None, None) => continue,
        };
        let applied = o.becomes_base.is_some();
        if holds && !applied {
            let mut def = becomes_creature(&base, &token, retain);
            if lost.is_some() {
                def.abilities.clear();
                (def.power, def.toughness) = (base.power, base.toughness);
            }
            let def = Arc::new(def);
            let o = game.obj_mut(id);
            o.becomes_base = Some(base);
            o.def = def;
        } else if !holds && applied {
            let o = game.obj_mut(id);
            o.def = o.becomes_base.take().unwrap();
        }
    }
    refresh_land_types(game);
}

/// `AddBasicLandTypeAllLandsEffect` (Urborg): each land is also the basic land types the statics give, with their
/// mana abilities (not a second one it already has). And an Aura's `GainAbilityAttachedEffect` of a mana, activated or
/// triggered ability (Old-Growth Troll, Curious Obsession) adds that ability to what it enchants.
fn refresh_land_types(game: &mut Game) {
    let mut types: Vec<String> = Vec::new();
    let mut granted: Vec<(ObjId, AbilityDef)> = Vec::new();
    for id in game.battlefield.iter() {
        for a in &game.obj(*id).def.abilities {
            if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                // an Aura that gives a keyword and an activated ability (Prophetic Ravings' haste and "{T}, Discard
                // a card: Draw a card."): Java numbers both, the keyword first, so it goes into the list too
                let gives_activated = effects.iter().any(|e| {
                    matches!(e, EffectDef::GainAbilityAttachedEffect { ability }
                        if ability.mana().is_some()
                            || matches!(**ability, AbilityDef::SimpleActivatedAbility { .. } | AbilityDef::TriggeredAbility { .. }))
                });
                for e in effects {
                    match e {
                        EffectDef::GainAbilityAttachedEffect { ability }
                            if gives_activated
                                && ability.mana().is_none()
                                && !matches!(
                                    **ability,
                                    AbilityDef::SimpleActivatedAbility { .. }
                                        | AbilityDef::TriggeredAbility { .. }
                                        | AbilityDef::WardAbility { .. }
                                ) =>
                        {
                            if let Some(to) = game.attached(*id) {
                                granted.push((to, (**ability).clone()));
                            }
                        }
                        EffectDef::AddBasicLandTypeAllLandsEffect { subtype } => {
                            if !types.contains(subtype) {
                                types.push(subtype.clone());
                            }
                        }
                        EffectDef::GainAbilityAttachedEffect { ability }
                            if ability.mana().is_some()
                                || matches!(
                                    **ability,
                                    AbilityDef::SimpleActivatedAbility { .. } | AbilityDef::TriggeredAbility { .. }
                                ) =>
                        {
                            if let Some(to) = game.attached(*id) {
                                granted.push((to, (**ability).clone()));
                            }
                        }
                        // Bootleggers' Stash: lands you control have "{T}: Create a Treasure token."
                        // Constricting Sliver: Slivers you control have "When this creature enters, …"
                        EffectDef::GainAbilityControlledEffect { ability, filter, exclude_source, .. }
                            if matches!(**ability, AbilityDef::SimpleActivatedAbility { .. })
                                && filter.0.iter().any(|p| matches!(p, Pred::Type { t: CardType::Land }))
                                || matches!(**ability, AbilityDef::TriggeredAbility { .. })
                                // Great Divide Guide: each land and Ally you control has "{T}: Add one mana of any color."
                                || ability.mana().is_some() =>
                        {
                            let you = game.obj(*id).controller;
                            for to in game.battlefield.iter() {
                                if !(*exclude_source && to == id)
                                    && game.obj(*to).controller == you
                                    && game.filter_matches(filter, you, Some(*id), *to)
                                {
                                    granted.push((*to, (**ability).clone()));
                                }
                            }
                        }
                        // Petrified Hamlet: lands with the chosen name have "{T}: Add {C}"; Food Fight: artifacts you
                        // control have "{2}, Sacrifice this artifact: …"
                        EffectDef::GainAbilityAllEffect { ability, filter, .. }
                            if ability.mana().is_some()
                                || matches!(**ability, AbilityDef::SimpleActivatedAbility { .. }) =>
                        {
                            let you = game.obj(*id).controller;
                            for to in game.battlefield.iter() {
                                if game.filter_matches(filter, you, Some(*id), *to) {
                                    granted.push((*to, (**ability).clone()));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    // a triggered ability a resolved effect gave until end of turn (Herd Heirloom's "Whenever this creature deals
    // combat damage to a player, draw a card"): Java adds it to the permanent, so it triggers like its own
    for g in game.grants.iter() {
        if matches!(g.ability, AbilityDef::TriggeredAbility { .. })
            && game.obj(g.target).zone == Zone::Battlefield
            && game.obj(g.target).zcc == g.zcc
        {
            granted.push((g.target, g.ability.clone()));
        }
    }
    // Tide Shaper's: kept while its source and the land are still those objects
    let here = |g: &Game, o: ObjId, z: u32| g.obj(o).zone == Zone::Battlefield && g.obj(o).zcc == z;
    if game.land_becomes.iter().any(|(l, lz, s, sz, _)| !here(game, *l, *lz) || !here(game, *s, *sz)) {
        let kept: Vec<_> = game
            .land_becomes
            .iter()
            .filter(|(l, lz, s, sz, _)| here(game, *l, *lz) && here(game, *s, *sz))
            .cloned()
            .collect();
        game.land_becomes = kept;
    }
    let ids: Vec<ObjId> = game.battlefield.iter().copied().collect();
    let mut reapply = false;
    for id in ids {
        // its def replaced by one from before the types applied (Mutavault animated before Urborg came, back to its
        // land at the end of the turn): that one is its base now, and the types apply again, as Java's layers would
        // (read under Leyline of the Guildpact's types, which go on top of these)
        let o = game.obj(id);
        let own = o.pre_guildpact.as_ref().unwrap_or(&o.def);
        let stale = o.land_type_base.as_ref().is_some_and(|(_, t, g)| {
            !t.iter().any(|x| x.starts_with('='))
                && (t.iter().any(|x| !own.abilities.contains(&land_type_mana(x)))
                    || g.iter().any(|a| !own.abilities.contains(a)))
        });
        if stale {
            game.obj_mut(id).land_type_base = None;
        }
        let o = game.obj(id);
        let base = o.land_type_base.as_ref().map(|(b, _, _)| b.clone()).unwrap_or_else(|| o.def.clone());
        let mut given: Vec<String> = if base.types.contains(&CardType::Land) { types.clone() } else { Vec::new() };
        // the latest "becomes", marked `=Type`: it replaces the land's types and abilities first
        if let Some((_, _, _, _, t)) = game.land_becomes.iter().rev().find(|(l, _, _, _, _)| *l == id) {
            if base.types.contains(&CardType::Land) {
                given.insert(0, format!("={t}"));
                given.insert(1, t.clone());
            }
        }
        let gained: Vec<AbilityDef> = granted.iter().filter(|(to, _)| *to == id).map(|(_, a)| a.clone()).collect();
        let (applied, applied_gained) =
            o.land_type_base.as_ref().map(|(_, t, g)| (t.clone(), g.clone())).unwrap_or_default();
        if given == applied && gained == applied_gained {
            continue;
        }
        // Leyline of the Guildpact's types go on top of these: taken off while the land is rebuilt, put back after
        if o.pre_guildpact.is_some() {
            game.unapply_guildpact(id);
            reapply = true;
        }
        let o = game.obj(id);
        let base = o.land_type_base.as_ref().map(|(b, _, _)| b.clone()).unwrap_or_else(|| o.def.clone());
        if given.is_empty() && gained.is_empty() {
            let o = game.obj_mut(id);
            let (base, types, gave) = o.land_type_base.take().unwrap();
            if types.iter().any(|t| t.starts_with('=')) {
                o.def = base;
                continue;
            }
            // what the types and effects gave it goes, what it became since stays (Mutavault animated after Urborg
            // came, when Urborg leaves)
            let mut def = (*o.def).clone();
            for t in &types {
                let a = land_type_mana(t);
                if !base.abilities.contains(&a) {
                    def.abilities.retain(|x| *x != a);
                }
                if !base.subtypes.contains(t) {
                    def.subtypes.retain(|x| x != t);
                }
            }
            for a in &gave {
                if let Some(k) = def.abilities.iter().rposition(|x| x == a) {
                    def.abilities.remove(k);
                }
            }
            o.def = Arc::new(def);
            continue;
        }
        let mut land = (*base).clone();
        // the land types' mana abilities come before what effects gave it (Java's layer 4 before layer 6: Urza's
        // Saga's chapter abilities follow Yavimaya's {T}: Add {G})
        let mut at = o.printed_def.as_ref().map(|p| p.abilities.len()).filter(|n| *n <= land.abilities.len());
        for t in &given {
            if t.starts_with('=') {
                land.subtypes.retain(|s| !["Plains", "Island", "Swamp", "Mountain", "Forest"].contains(&s.as_str()));
                land.abilities.clear();
                at = None;
                continue;
            }
            if !land.subtypes.contains(t) {
                land.subtypes.push(t.clone());
            }
            let ability = land_type_mana(t);
            if !land.abilities.contains(&ability) {
                match at.as_mut() {
                    Some(n) => {
                        land.abilities.insert(*n, ability);
                        *n += 1;
                    }
                    None => land.abilities.push(ability),
                }
            }
        }
        land.abilities.extend(gained.iter().cloned());
        let o = game.obj_mut(id);
        o.land_type_base = Some((base, given, gained));
        o.def = Arc::new(land);
    }
    if reapply {
        game.update_guildpact();
    }
}

/// A basic land type's mana ability
fn land_type_mana(t: &str) -> AbilityDef {
    let (effects, cost) = (Vec::new(), Vec::new());
    match t {
        "Plains" => AbilityDef::WhiteManaAbility { effects, cost },
        "Island" => AbilityDef::BlueManaAbility { effects, cost },
        "Swamp" => AbilityDef::BlackManaAbility { effects, cost },
        "Mountain" => AbilityDef::RedManaAbility { effects, cost },
        _ => AbilityDef::GreenManaAbility { effects, cost },
    }
}

/// A spell's targets, as it was cast: its chosen modes' (or its own), the kicked or gifted ones if so
fn spell_target_defs(def: &CardDef, item: &StackItem) -> Vec<TargetDef> {
    let mut defs: Vec<TargetDef> = Vec::new();
    for k in if item.modes.is_empty() { vec![0] } else { item.modes.clone() } {
        defs.extend(def.spell.mode(k).1.iter().cloned());
    }
    if let Some((unkicked, kicked)) = &def.spell.kicked_targets {
        defs = if item.kicked { kicked.clone() } else { unkicked.clone() };
    }
    if let Some((plain, promised)) = &def.spell.gift_targets {
        defs = if item.gift_promised { promised.clone() } else { plain.clone() };
    }
    defs
}

/// `Spell.createSingleCopy` with new targets: the copy (a token) goes on the stack, then `chooseNewTargets` asks of
/// each target whether to change it (`chooseNewTarget`)
fn copy_spell(game: &mut Game, d: &mut dyn Decider, original: &StackItem, controller: Seat) {
    let def = game.obj(original.card).def.clone();
    let copy = game.create_obj(def.clone(), controller, Zone::Stack);
    {
        let o = game.obj_mut(copy);
        o.token = true;
        o.trigger_order = (1 << 32) + copy.0 as u64;
    }
    let mut item = original.clone();
    item.card = copy;
    item.controller = controller;
    item.ward_of = None;
    Arc::make_mut(&mut game.stack).push(item.clone());
    let defs = spell_target_defs(&def, original);
    let mut targets = item.targets.clone();
    let mut zccs = item.target_zcc.clone();
    for (k, tdef) in defs.iter().enumerate() {
        let Some(old) = original.targets.get(k).cloned() else { continue };
        let total = old.len();
        let mut new: Vec<Target> = Vec::new();
        // a spell or ability target that left the stack: Java finds nothing by its id (`getNameOftarget`), so it
        // keeps it without asking
        let stack_target = matches!(
            tdef,
            TargetDef::TargetSpell { .. }
                | TargetDef::TargetStackAbility { .. }
                | TargetDef::TargetTriggeredAbilityOrColorlessSpell
        );
        for (n, t) in old.iter().enumerate() {
            if let Target::Obj(o) = t {
                let on_stack = game.stack.iter().any(|i| i.card == *o) || game.resolving.map(|(_, c)| c) == Some(*o);
                if stack_target && !on_stack {
                    new.push(*t);
                    continue;
                }
            }
            let q = Query::YesNo { message: format!("Change this {} of {} target?", n + 1, total) };
            let change = loop {
                match ask(game, d, controller, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return,
                    _ => {}
                }
            };
            if !change {
                new.push(*t);
                continue;
            }
            // one at a time; nothing chosen, or a target already chosen, asks whether to keep the old one
            let single = tdef.single();
            loop {
                let chosen = choose_target(game, d, controller, &single, Some(copy), "", false);
                if d.stopped() {
                    return;
                }
                let picked = chosen.as_ref().and_then(|c| c.first().copied());
                let reset = match picked {
                    None => true,
                    Some(p) => new.contains(&p) || old.contains(&p),
                };
                if !reset {
                    new.push(picked.unwrap());
                    break;
                }
                let q = Query::YesNo { message: "Reset to original target?".into() };
                let yes = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if yes {
                    new.push(*t);
                    break;
                }
            }
        }
        if let Some(slot) = targets.get_mut(k) {
            *slot = new.clone();
        }
        if let Some(slot) = zccs.get_mut(k) {
            *slot = new
                .iter()
                .map(|t| match t {
                    Target::Obj(o) => game.obj(*o).zcc,
                    Target::Player(_) => 0,
                })
                .collect();
        }
    }
    if let Some(top) = Arc::make_mut(&mut game.stack).iter_mut().rev().find(|i| i.card == copy) {
        top.targets = targets;
        top.target_zcc = zccs;
    }
}

/// `CreateTokenCopyTargetEffect`: a token copy of `of`'s copiable values (its printed card), with a set power and
/// toughness
fn token_copy(game: &mut Game, d: &mut dyn Decider, of: ObjId, controller: Seat, pt: Option<(i32, i32)>) {
    token_copy_with(game, d, of, controller, pt, false);
}

/// `token_copy`, the tokens gaining haste if asked; the tokens made
fn token_copy_with(
    game: &mut Game,
    d: &mut dyn Decider,
    of: ObjId,
    controller: Seat,
    pt: Option<(i32, i32)>,
    haste: bool,
) -> Vec<ObjId> {
    let o = game.obj(of);
    let mut def = (*o.copiable()).clone();
    if let Some((p, t)) = pt {
        def.remove_pt_cda();
        def.power = p;
        def.toughness = t;
    }
    if haste {
        def.abilities.push(AbilityDef::HasteAbility);
    }
    let def = Arc::new(def);
    let mut made = Vec::new();
    for _ in 0..token_copies(game, controller) {
        let id = game.create_obj(def.clone(), controller, Zone::Library);
        let o = game.obj_mut(id);
        o.token = true;
        o.trigger_order = (1 << 32) + id.0 as u64;
        enter_battlefield(game, d, id, controller, false);
        made.push(id);
    }
    made
}

/// `GiftType.applyGift`: the opponent gets the token (a tapped Fish), or draws a card
fn give_gift(game: &mut Game, d: &mut dyn Decider, def: &CardDef, to: Seat, source: ObjId) {
    for a in &def.abilities {
        if let AbilityDef::GiftAbility { token, .. } = a {
            match token {
                Some(token) => {
                    let tdef = Arc::new(token.card());
                    token_image(game, token, source);
                    for _ in 0..token_copies(game, to) {
                        let id = game.create_obj(tdef.clone(), to, Zone::Library);
                        let o = game.obj_mut(id);
                        o.token = true;
                        o.trigger_order = (1 << 32) + id.0 as u64;
                        enter_battlefield(game, d, id, to, false);
                        game.obj_mut(id).tapped = true;
                    }
                }
                None => draw(game, to, 1),
            }
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
                .filter(|(t, z)| target_legal(game, item.controller, def, Some(item.card), *t, *z))
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
            match ($a, item.lki_power) {
                (Amount::Of(crate::card::Value::SourcePower), Some(p)) => p,
                (a, _) => game.value(a, item.controller, source.0, item.x),
            }
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
            EffectDef::TargetPointerEffect { target, effect } => {
                let only = vec![legal.get(*target).cloned().unwrap_or_default()];
                apply_each_effect(game, d, item, std::slice::from_ref(&**effect), &only, source);
            }
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
            EffectDef::CounterTargetExileEffect => {
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone == Zone::Stack && !cant_be_countered(game, o) {
                        note_countered(game, o, item.card);
                        Arc::make_mut(&mut game.stack).retain(|i| i.card != o || i.ability.is_some());
                        game.move_to(o, Zone::Exile);
                    }
                }
            }
            EffectDef::CounterTargetEffect => {
                // a triggered ability (Consign to Memory): it's removed from the stack
                if let (Some(Target::Obj(o)), Some(TargetDef::TargetTriggeredAbilityOrColorlessSpell)) =
                    (target, game.item_def(item).spell.targets.first())
                {
                    if let Some(k) =
                        game.stack.iter().rposition(|i| i.card == o && (i.ability.is_some() || i.delayed.is_some()))
                    {
                        Arc::make_mut(&mut game.stack).remove(k);
                        continue;
                    }
                }
                // an ability whose source isn't a spell (Defabricate's second mode): the ability leaves the stack
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone != Zone::Stack {
                        if let Some(k) =
                            game.stack.iter().rposition(|i| i.card == o && (i.ability.is_some() || i.delayed.is_some()))
                        {
                            Arc::make_mut(&mut game.stack).remove(k);
                            continue;
                        }
                    }
                }
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone == Zone::Stack && !cant_be_countered(game, o) {
                        note_countered(game, o, item.card);
                        Arc::make_mut(&mut game.stack).retain(|i| i.card != o || i.ability.is_some());
                        game.move_to(o, Zone::Graveyard);
                    }
                }
            }
            EffectDef::BoostTargetEffect { power, toughness, duration } => {
                let power = &v!(power);
                let toughness = &v!(toughness);
                for o in &objs {
                    boost(game, *o, *power, *toughness, *duration);
                }
            }
            EffectDef::BoostSourceEffect { power, toughness, duration } => {
                let power = &v!(power);
                let toughness = &v!(toughness);
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
                        if counter.amount == 1 {
                            k
                        } else {
                            counter.amount + k.max(0)
                        }
                    }
                };
                let (o, zcc) = source;
                if n > 0 && game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    if counter.name == "lore" {
                        add_lore(game, o, n);
                    } else {
                        put_counters(game, o, &counter.name, n, controller);
                    }
                }
            }
            EffectDef::AdaptEffect { amount } => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc && game.obj(o).counter("+1/+1") == 0
                {
                    put_counters(game, o, "+1/+1", *amount, controller);
                }
            }
            EffectDef::AddCountersTargetEffect { counter, value, second } => {
                let n = match value {
                    Some(a) => v!(a),
                    None => counter.amount,
                };
                let to: Vec<ObjId> = if *second {
                    legal.get(1).map_or(Vec::new(), |ts| {
                        ts.iter().filter_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }).collect()
                    })
                } else {
                    objs.clone()
                };
                if n > 0 {
                    for o in &to {
                        put_counters(game, *o, &counter.name, n, controller);
                    }
                }
            }
            EffectDef::RemoveCounterTargetEffect { counter } => {
                for o in &objs {
                    if game.obj(*o).counter(counter) > 0 {
                        let ob = game.obj_mut(*o);
                        ob.add_counters(counter, -1);
                        ob.counters.retain(|(_, k)| *k > 0);
                    }
                }
            }
            EffectDef::DarkSalvationEffect { token } => {
                if let Some(Target::Player(p)) = legal_target(0) {
                    let def = Arc::new(token.card());
                    let n = item.x.max(0) * token_copies(game, p) as i32;
                    if n > 0 {
                        token_image(game, token, source.0);
                    }
                    for _ in 0..n {
                        let id = game.create_obj(def.clone(), p, Zone::Library);
                        let o = game.obj_mut(id);
                        o.token = true;
                        o.trigger_order = (1 << 32) + id.0 as u64;
                        enter_battlefield(game, d, id, p, false);
                    }
                    if let Some(Target::Obj(o)) = legal_target(1) {
                        let n = game
                            .battlefield
                            .iter()
                            .filter(|z| game.obj(**z).controller == p && game.has_subtype(**z, "Zombie"))
                            .count() as i32;
                        if game.obj(o).zone == Zone::Battlefield {
                            boost(game, o, -n, -n, Duration::EndOfTurn);
                        }
                    }
                }
            }
            EffectDef::CreateTokenEffect {
                token,
                amount,
                tapped_and_attacking,
                tapped,
                attacking,
                sacrifice_at_end_of_combat,
                sacrifice_at_next_end_step,
                haste_until_end_of_turn,
                also,
                attach_source,
            } => {
                let amount = &(v!(amount) * token_copies(game, controller) as i32);
                let def = Arc::new(token.card());
                // `TokenImpl.putOntoBattlefield`: none to make, no image picked
                if *amount > 0 {
                    token_image(game, token, source.0);
                }
                let mut created = Vec::new();
                for _ in 0..*amount {
                    let id = game.create_obj(def.clone(), controller, Zone::Library);
                    let o = game.obj_mut(id);
                    o.token = true;
                    // registered after every card
                    o.trigger_order = (1 << 32) + id.0 as u64;
                    enter_battlefield(game, d, id, controller, false);
                    if *tapped_and_attacking || *attacking {
                        // `Combat.addAttackingCreature`: with planeswalkers or battles to attack too, asked which
                        let defender = game.opponent(controller);
                        let Some(pw) = choose_defender(game, d, controller, defender) else { return };
                        let o = game.obj_mut(id);
                        o.tapped |= *tapped_and_attacking;
                        o.attacking = Some(defender);
                        o.attacking_pw = pw;
                        join_combat(game, id);
                    }
                    if *tapped {
                        game.obj_mut(id).tapped = true;
                    }
                    if *haste_until_end_of_turn {
                        grant(game, id, &AbilityDef::HasteAbility);
                    }
                    created.push((id, game.obj(id).zcc));
                }
                if let Some(optional) = attach_source {
                    let (src, zcc) = source;
                    if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                        let attach = !*optional || {
                            let q = Query::YesNo { message: format!("Attach {} to the token?", game.obj(src).name()) };
                            loop {
                                match ask(game, d, controller, &q) {
                                    Answer::Bool(b) => break b,
                                    Answer::Stop => return,
                                    _ => {}
                                }
                            }
                        };
                        let tokens: Vec<ObjId> = created
                            .iter()
                            .filter(|(t, z)| game.obj(*t).zone == Zone::Battlefield && game.obj(*t).zcc == *z)
                            .map(|(t, _)| *t)
                            .collect();
                        if attach && tokens.len() == 1 {
                            let z = game.obj(tokens[0]).zcc;
                            game.attach_to(src, tokens[0], z);
                        }
                    }
                }
                if *sacrifice_at_end_of_combat {
                    let delayed = Delayed {
                        rule: "At end of combat, sacrifice that token.".into(),
                        sacrifice: created.clone(),
                        warp_exile: Vec::new(),
                        effects: Vec::new(),
                        source_zcc: 0,
                        fixed: Vec::new(),
                        return_tapped: Vec::new(),
                        targets: Vec::new(),
                        condition: None,
                    };
                    game.at_end_of_combat
                        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
                }
                if *sacrifice_at_next_end_step {
                    let delayed = Delayed {
                        rule: "At the beginning of the next end step, sacrifice those tokens.".into(),
                        sacrifice: created,
                        warp_exile: Vec::new(),
                        effects: Vec::new(),
                        source_zcc: 0,
                        fixed: Vec::new(),
                        return_tapped: Vec::new(),
                        targets: Vec::new(),
                        condition: None,
                    };
                    game.at_next_end_step
                        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
                }
                if let Some(t) = also {
                    let e = EffectDef::CreateTokenEffect {
                        token: t.clone(),
                        amount: Amount::Fixed(1),
                        tapped_and_attacking: false,
                        tapped: false,
                        attacking: false,
                        sacrifice_at_end_of_combat: false,
                        sacrifice_at_next_end_step: false,
                        haste_until_end_of_turn: false,
                        also: None,
                        attach_source: None,
                    };
                    apply_effects(game, d, item, std::slice::from_ref(&e), legal, source);
                }
            }
            EffectDef::EarthbendTargetEffect { amount } => {
                let n = v!(amount);
                let Some(Target::Obj(o)) = target else { continue };
                if game.obj(o).zone != Zone::Battlefield {
                    continue;
                }
                // a 0/0 creature with haste that's still a land, until it leaves (`BecomesCreatureTargetEffect`)
                let earthbent = |old: &CardDef| {
                    let mut def = old.clone();
                    if !def.types.contains(&CardType::Creature) {
                        def.types.push(CardType::Creature);
                    }
                    def.power = 0;
                    def.toughness = 0;
                    def.abilities.push(AbilityDef::HasteAbility);
                    Arc::new(def)
                };
                let old = game.obj(o).def.clone();
                let zcc = game.obj(o).zcc;
                let p = game.obj_mut(o);
                // the card as printed: under an animation until end of turn (Mutavault's), what it was before
                if p.printed_def.is_none() {
                    p.printed_def = Some(p.until_end_of_turn_def.clone().unwrap_or_else(|| old.clone()));
                }
                p.def = earthbent(&old);
                // still one once an animation until end of turn ends (Mutavault's)
                if let Some(eot) = p.until_end_of_turn_def.take() {
                    p.until_end_of_turn_def = Some(earthbent(&eot));
                }
                if n > 0 {
                    p.add_counters("+1/+1", n);
                }
                game.earthbent.push((o, zcc));
            }
            EffectDef::EmpowerJaceEffect { amount, token } => {
                let jaces = |game: &Game| -> Vec<ObjId> {
                    game.battlefield
                        .iter()
                        .copied()
                        .filter(|o| {
                            let ob = game.obj(*o);
                            ob.controller == controller
                                && ob.token
                                && game.has_type(*o, CardType::Planeswalker)
                                && ob.def.subtypes.iter().any(|s| s == "Jace")
                        })
                        .collect()
                };
                let mut found = jaces(game);
                if found.is_empty() {
                    let e = EffectDef::CreateTokenEffect {
                        token: token.clone(),
                        amount: Amount::Fixed(1),
                        tapped_and_attacking: false,
                        tapped: false,
                        attacking: false,
                        sacrifice_at_end_of_combat: false,
                        sacrifice_at_next_end_step: false,
                        haste_until_end_of_turn: false,
                        also: None,
                        attach_source: None,
                    };
                    apply_effects(game, d, item, std::slice::from_ref(&e), legal, source);
                    found = jaces(game);
                }
                let chosen = match found.len() {
                    0 => None,
                    1 => Some(found[0]),
                    _ => match choose_cards(game, d, controller, &found, &found, 1, 1, "empower") {
                        Some(c) => c.first().copied(),
                        None => return,
                    },
                };
                let n = v!(amount);
                if let Some(j) = chosen {
                    game.obj_mut(j).add_counters("loyalty", n);
                }
            }
            EffectDef::CreateRoleAttachedTargetEffect { token } => {
                let def = Arc::new(token.card());
                for o in &objs {
                    if game.obj(*o).zone != Zone::Battlefield {
                        continue;
                    }
                    token_image(game, token, source.0);
                    let id = game.create_obj(def.clone(), controller, Zone::Library);
                    let t = game.obj_mut(id);
                    t.token = true;
                    t.trigger_order = (1 << 32) + id.0 as u64;
                    let zcc = game.obj(*o).zcc;
                    game.attach_to(id, *o, zcc);
                    enter_battlefield(game, d, id, controller, false);
                    game.attach_to(id, *o, zcc);
                }
            }
            EffectDef::CreateNextCreatureSpellDelayedEffect { amount, rule } => {
                let delayed = Delayed {
                    rule: rule.clone(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: vec![EffectDef::AddCounterNextSpellEffect { amount: *amount }],
                    source_zcc: source.1,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                let turn = game.turn;
                game.next_creature_spell.push((
                    controller,
                    turn,
                    StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) },
                ));
            }
            EffectDef::AddCounterNextSpellEffect { amount } => {
                for o in &objs {
                    if game.obj(*o).zone == Zone::Stack {
                        let zcc = game.obj(*o).zcc;
                        game.enter_counters.push((*o, zcc, *amount));
                    }
                }
            }
            EffectDef::ExileSagaAndReturnTransformedEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    game.move_to(src, Zone::Exile);
                    if let Some(back) = game.obj(src).def.transform_back.clone() {
                        let o = game.obj_mut(src);
                        o.front = Some(std::mem::replace(&mut o.def, back));
                        enter_battlefield(game, d, src, controller, false);
                    }
                }
            }
            EffectDef::TishanasTidebinderEffect => {
                let (src, zcc) = source;
                for o in &objs {
                    let Some(k) =
                        game.stack.iter().rposition(|i| i.card == *o && (i.ability.is_some() || i.delayed.is_some()))
                    else {
                        continue;
                    };
                    Arc::make_mut(&mut game.stack).remove(k);
                    let p = game.obj(*o);
                    let kinds = [CardType::Artifact, CardType::Creature, CardType::Planeswalker];
                    if p.zone != Zone::Battlefield
                        || !kinds.iter().any(|t| game.has_type(*o, *t))
                        || game.obj(src).zone != Zone::Battlefield
                        || game.obj(src).zcc != zcc
                        || p.lost_abilities.is_some()
                    {
                        continue;
                    }
                    lose_all_abilities(game, *o, src, zcc);
                }
            }
            EffectDef::LoseAllAbilitiesUntilSourceLeavesEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone != Zone::Battlefield || game.obj(src).zcc != zcc {
                    continue;
                }
                for o in &objs {
                    if game.obj(*o).zone != Zone::Battlefield || game.obj(*o).lost_abilities.is_some() {
                        continue;
                    }
                    lose_all_abilities(game, *o, src, zcc);
                }
            }
            EffectDef::FirebendingEffect { amount } => {
                let p = game.player_mut(controller);
                p.mana_pool.colored[Color::R.index()] += *amount;
                p.combat_red += *amount;
            }
            EffectDef::AddManaOfAnyColorEffect { amount } => {
                let names = ["White", "Blue", "Black", "Red", "Green"];
                let choices: Vec<String> = names.iter().map(|n| n.to_string()).collect();
                let q = Query::Choice { message: format!("Select a color of mana to add {amount} of it"), choices };
                let color = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Text(t) => {
                            if let Some(i) = names.iter().position(|n| *n == t) {
                                break Some(Color::ALL[i]);
                            }
                        }
                        Answer::Stop => return,
                        _ => {}
                    }
                    if d.stopped() {
                        return;
                    }
                };
                if let Some(c) = color {
                    game.player_mut(controller).mana_pool.colored[c.index()] += *amount;
                }
            }
            EffectDef::GarrukCurseBreakerEffect => {
                let delayed = Delayed {
                    // `UntilYourNextTurnDelayedTriggeredAbility.getRule`; BoostTargetEffect describes a fixed target as "it"
                    rule: "Until your next turn, whenever one or more creatures attack one of your opponents, it gets \
                           +2/+2 until end of turn. Those creatures get +2/+2 and gain trample until end of turn."
                        .into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: vec![
                        EffectDef::BoostTargetEffect {
                            power: Amount::Fixed(2),
                            toughness: Amount::Fixed(2),
                            duration: Duration::EndOfTurn,
                        },
                        EffectDef::GainAbilityTargetEffect {
                            ability: Box::new(AbilityDef::TrampleAbility),
                            duration: Duration::EndOfTurn,
                        },
                    ],
                    source_zcc: source.1,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                game.attacks_opponent.push((
                    controller,
                    StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) },
                ));
            }
            EffectDef::LivelyDirgeEffect => {
                // `LivelyDirgeTarget`: what's left to choose keeps the total at 4 or less
                let mut chosen: Vec<ObjId> = Vec::new();
                loop {
                    if chosen.len() >= 2 {
                        break;
                    }
                    let total: u32 = chosen.iter().map(|c| game.obj(*c).def.mana_value()).sum();
                    let left: Vec<ObjId> = game
                        .player(controller)
                        .graveyard
                        .iter()
                        .copied()
                        .filter(|c| {
                            !chosen.contains(c)
                                && game.obj(*c).is(CardType::Creature)
                                && total + game.obj(*c).def.mana_value() <= 4
                        })
                        .collect();
                    match ask(game, d, controller, &Query::Cards { objs: left.clone(), required: false }) {
                        Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                        Answer::Obj(o) if left.contains(&o) => {
                            chosen.push(o);
                            // `isChoiceCompleted`: no other card fits in what's left of the total
                            let total: u32 = chosen.iter().map(|c| game.obj(*c).def.mana_value()).sum();
                            let more = game.player(controller).graveyard.iter().any(|c| {
                                !chosen.contains(c)
                                    && game.obj(*c).is(CardType::Creature)
                                    && total + game.obj(*c).def.mana_value() <= 4
                            });
                            if !more {
                                break;
                            }
                        }
                        Answer::Stop => return,
                        Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                        _ => break,
                    }
                    if d.stopped() {
                        return;
                    }
                }
                for c in chosen {
                    enter_battlefield(game, d, c, controller, false);
                }
            }
            EffectDef::OffspringEffect => {
                // of the permanent (or as it last was), 1/1
                token_copy(game, d, source.0, controller, Some((1, 1)));
            }
            EffectDef::ReflectionOfKikiJikiEffect => {
                let Some(of) = objs.first().copied() else { return };
                let made = token_copy_with(game, d, of, controller, None, true);
                let delayed = Delayed {
                    rule: "At the beginning of the next end step, sacrifice the token copies.".into(),
                    sacrifice: made.iter().map(|t| (*t, game.obj(*t).zcc)).collect(),
                    warp_exile: Vec::new(),
                    effects: Vec::new(),
                    source_zcc: 0,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                game.at_next_end_step
                    .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
            }
            EffectDef::TheScarabGodExileEffect => {
                let Some(card) = objs.first().copied() else { return };
                game.move_to(card, Zone::Exile);
                let mut def = (*game.obj(card).copiable()).clone();
                def.remove_pt_cda();
                def.power = 4;
                def.toughness = 4;
                def.subtypes = vec!["Zombie".into()];
                def.colors = vec![Color::B];
                let def = Arc::new(def);
                for _ in 0..token_copies(game, controller) {
                    let id = game.create_obj(def.clone(), controller, Zone::Library);
                    let o = game.obj_mut(id);
                    o.token = true;
                    o.trigger_order = (1 << 32) + id.0 as u64;
                    enter_battlefield(game, d, id, controller, false);
                }
            }
            EffectDef::ReturnSourceToHandNextEndStepEffect => {
                let card = source.0;
                let delayed = Delayed {
                    rule: "At the beginning of the next end step, return that creature to its owner's hand.".into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: vec![EffectDef::ReturnToHandTargetEffect],
                    source_zcc: source.1,
                    fixed: vec![(card, game.obj(card).zcc)],
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                game.at_next_end_step
                    .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(card, controller) });
            }
            EffectDef::CreateCopyNextSpellDelayedEffect => {
                let delayed = Delayed {
                    rule: "When you next cast an instant or sorcery spell this turn, copy that spell. You may choose \
                           new targets for the copy."
                        .into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: vec![EffectDef::CopyTargetSpellEffect],
                    source_zcc: source.1,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                let turn = game.turn;
                game.next_instant_sorcery.push((
                    controller,
                    turn,
                    StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) },
                ));
            }
            EffectDef::CopyTargetSpellEffect => {
                for o in &objs {
                    let original =
                        game.stack.iter().find(|i| i.card == *o && i.ability.is_none() && i.delayed.is_none()).cloned();
                    if let Some(original) = original {
                        copy_spell(game, d, &original, controller);
                    }
                }
            }
            EffectDef::ReplicateCopyEffect => {
                let (spell, zcc) = source;
                let Some(original) =
                    game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
                else {
                    continue;
                };
                if game.obj(spell).zcc != zcc {
                    continue;
                }
                for _ in 0..original.replicated {
                    copy_spell(game, d, &original, controller);
                    if d.stopped() {
                        return;
                    }
                }
            }
            EffectDef::CopySourceSpellEffect => {
                let (spell, zcc) = source;
                let Some(original) =
                    game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
                else {
                    continue;
                };
                if game.obj(spell).zcc != zcc {
                    continue;
                }
                copy_spell(game, d, &original, controller);
            }
            EffectDef::ChandraTorchOfDefianceEffect => {
                // exile the top card; you may cast it (a nonland card), and if you don't, 2 damage to each opponent
                let Some(card) = game.player(controller).library.first().copied() else { continue };
                game.move_to(card, Zone::Exile);
                let mut cast = false;
                if !game.obj(card).is(CardType::Land) {
                    let q = Query::YesNo {
                        message: format!("Cast {}? (You still pay the costs)", game.obj(card).def.name),
                    };
                    let yes = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    if yes && game.obj(card).zone == Zone::Exile {
                        cast = cast_allowed(game, d, controller, card);
                        if d.stopped() {
                            return;
                        }
                    }
                }
                if !cast {
                    for p in 0..game.seats() {
                        if p != controller {
                            deal_damage(game, source.0, Target::Player(p), 2, false);
                        }
                    }
                }
            }
            EffectDef::SpellQuellerEntersEffect => {
                // the spell leaves the stack into this permanent's exile zone
                let (src, zcc) = source;
                for o in &objs {
                    let Some(k) =
                        game.stack.iter().position(|i| i.card == *o && i.ability.is_none() && i.delayed.is_none())
                    else {
                        continue;
                    };
                    Arc::make_mut(&mut game.stack).remove(k);
                    game.move_to(*o, Zone::Exile);
                    if !game.obj(*o).token {
                        game.exiled_for_source.push((src, zcc, *o));
                    }
                }
            }
            EffectDef::AcademicDisputeEffect => {
                for o in &objs {
                    let q = Query::YesNo {
                        message: format!("Have {} gain reach until end of turn?", game.obj(*o).def.name),
                    };
                    let yes = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    if yes {
                        grant(game, *o, &AbilityDef::ReachAbility);
                    }
                }
            }
            EffectDef::LearnEffect => {
                // `WishEffect` asks first; a yes with no Lesson outside the game does nothing more
                let q = Query::YesNo { message: "Choose a Lesson card from outside the game?".into() };
                let wish = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if wish {
                    // `WishEffect`: one of the Lessons outside the game, into the hand
                    let outside = game.player(controller).outside.clone();
                    let lessons: Vec<ObjId> =
                        outside.iter().copied().filter(|c| game.has_subtype(*c, "Lesson")).collect();
                    if !lessons.is_empty() {
                        let rule_text = item_rule(game, item);
                        let Some(chosen) = choose_cards(game, d, controller, &outside, &lessons, 1, 1, &rule_text)
                        else {
                            return;
                        };
                        for c in chosen {
                            game.move_to(c, Zone::Hand);
                        }
                    }
                } else {
                    let rummage: EffectDef = serde_json::from_str(
                        r#"{"class": "DoIfCostPaid", "effect": {"class": "DrawCardSourceControllerEffect", "amount": 1},
                            "cost": [{"class": "DiscardCardCost"}]}"#,
                    )
                    .unwrap();
                    apply_effects(game, d, item, std::slice::from_ref(&rummage), legal, source);
                }
            }
            EffectDef::CruelclawsHeistEffect => {
                // the opponent reveals their hand; you choose a nonland card, exiled (and yours to cast, with mana of
                // any type, if the gift was promised)
                if let Some(Target::Player(p)) = target {
                    let hand = game.player(p).hand.clone();
                    let possible: Vec<ObjId> =
                        hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
                    if !possible.is_empty() {
                        // Java's `TargetCardInHand` takes only cards in the chooser's own hand: none of these, so
                        // nothing is chosen (asked until the player gives up)
                        let rule_text = item_rule(game, item);
                        let Some(chosen) = choose_cards(game, d, controller, &hand, &[], 1, 1, &rule_text) else {
                            return;
                        };
                        for c in chosen {
                            game.move_to(c, Zone::Exile);
                            if item.gift_promised && game.obj(c).zone == Zone::Exile {
                                let o = game.obj_mut(c);
                                o.may_play = Some((controller, u32::MAX));
                                o.any_mana = true;
                            }
                        }
                    }
                }
            }
            EffectDef::ChooseLandNameEffect => {
                let q = Query::Choice { message: "Land card name".into(), choices: Vec::new() };
                let chosen = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Text(t) => break t,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if game.obj(source.0).zone == Zone::Battlefield {
                    game.obj_mut(source.0).chosen_name = Some(chosen);
                }
            }
            EffectDef::EsperOriginsEffect => {
                let id = source.0;
                if game.obj(id).zone == Zone::Stack && game.obj(id).cast_from == Some(Zone::Graveyard) {
                    game.move_to(id, Zone::Exile);
                    if let Some(back) = game.obj(id).def.transform_back.clone() {
                        let o = game.obj_mut(id);
                        o.front = Some(std::mem::replace(&mut o.def, back));
                        // (no finality counter: Java sets it for the front's id, and the back face enters)
                        enter_battlefield(game, d, id, game.obj(id).owner, false);
                    }
                }
            }
            EffectDef::SummonEsperMaduinEffect => {
                if let Some(c) = game.player(controller).library.first().copied() {
                    let o = game.obj(c);
                    if !o.is(CardType::Instant) && !o.is(CardType::Sorcery) {
                        game.move_to(c, Zone::Hand);
                    }
                }
            }
            EffectDef::BecomePreparedSourceEffect => {
                if game.obj(source.0).zcc == source.1 {
                    become_prepared(game, source.0);
                }
            }
            EffectDef::UnholyAnnexDrainEffect => {
                for p in 0..game.seats() {
                    if p != controller {
                        lose_life(game, p, 2);
                    }
                }
                gain_life(game, controller, 2);
            }
            EffectDef::GoblinGuideEffect => {
                let defender = game.opponent(controller);
                if let Some(c) = game.player(defender).library.first().copied() {
                    let zcc = game.obj(source.0).zcc;
                    game.revealed.push(crate::state::Revealed {
                        source: source.0,
                        zcc,
                        paren: false,
                        cards: vec![(c, Zone::Library)],
                    });
                    if game.obj(c).is(CardType::Land) {
                        game.move_to(c, Zone::Hand);
                    }
                }
            }
            EffectDef::RagavanNimblePilfererEffect => {
                if let Some(Target::Player(p)) = target {
                    if let Some(c) = game.player(p).library.first().copied() {
                        game.move_to(c, Zone::Exile);
                        if game.obj(c).zone == Zone::Exile {
                            let turn = game.turn;
                            let o = game.obj_mut(c);
                            o.may_play = Some((controller, turn));
                            o.cast_only = true;
                        }
                    }
                }
            }
            EffectDef::LaughingJasperFlintEffect => {
                if let Some(Target::Player(p)) = target {
                    let outlaws = ["Assassin", "Mercenary", "Pirate", "Rogue", "Warlock"];
                    let n = game
                        .battlefield
                        .iter()
                        .filter(|b| {
                            game.obj(**b).controller == controller && outlaws.iter().any(|t| game.has_subtype(**b, t))
                        })
                        .count();
                    let cards: Vec<ObjId> = game.player(p).library.iter().take(n).copied().collect();
                    for c in cards {
                        game.move_to(c, Zone::Exile);
                        if game.obj(c).zone == Zone::Exile {
                            let turn = game.turn;
                            let o = game.obj_mut(c);
                            o.may_play = Some((controller, turn));
                            o.any_mana = true;
                            o.cast_only = true;
                        }
                    }
                }
            }
            EffectDef::OldGrowthTrollReturnEffect { token } => {
                // it returns as an Aura enchantment (its other abilities stay) with enchant Forest you control
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Graveyard && game.obj(o).zcc == zcc {
                    let original = game.obj(o).def.clone();
                    let mut aura = (*original).clone();
                    aura.types = vec![CardType::Enchantment];
                    aura.subtypes = vec!["Aura".to_string()];
                    aura.spell.targets = vec![serde_json::from_value(serde_json::json!({
                        "class": "TargetPermanent",
                        "filter": [{"p": "subtype", "s": "Forest"}, {"p": "controller", "who": "you"}],
                    }))
                    .unwrap()];
                    // (Java adds an `EnchantAbility` first: only its rule line shows)
                    aura.abilities.push(
                        serde_json::from_value(serde_json::json!({
                            "class": "SimpleStaticAbility", "rule": "Enchant Forest you control", "effects": [],
                        }))
                        .unwrap(),
                    );
                    aura.abilities.push(
                        serde_json::from_value(serde_json::json!({
                            "class": "SimpleStaticAbility",
                            "rule": "Enchanted Forest has \"{T}: Add {G}{G}\" and \"{1}, {T}, Sacrifice this land: Create a tapped 4/4 green Troll Warrior creature token with trample.\"",
                            "effects": [
                                {"class": "GainAbilityAttachedEffect",
                                 "ability": {"class": "ColoredManaAbility", "color": "G", "amount": 2,
                                             "rule": "{T}: Add {G}{G}."}},
                                {"class": "GainAbilityAttachedEffect",
                                 "ability": {"class": "SimpleActivatedAbility",
                                             "cost": [{"class": "ManaCost", "cost": "{1}"}, {"class": "TapSourceCost"},
                                                      {"class": "SacrificeSourceCost"}],
                                             "effects": [{"class": "CreateTokenEffect", "token": token, "amount": 1,
                                                          "tapped": true}],
                                             "rule": "{1}, {T}, Sacrifice this land: Create a tapped 4/4 green Troll Warrior creature token with trample."}},
                            ],
                        }))
                        .unwrap(),
                    );
                    game.obj_mut(o).def = Arc::new(aura);
                    enter_battlefield(game, d, o, controller, false);
                    if game.obj(o).zone == Zone::Battlefield {
                        game.obj_mut(o).printed_def = Some(original);
                    } else {
                        game.obj_mut(o).def = original;
                    }
                }
            }
            EffectDef::RottenmouthViperEffect => {
                // `CountersSourceCount`: the counters it had as it left, when it's gone
                let viper = game.obj(source.0);
                let repeat = if viper.zone == Zone::Battlefield {
                    viper.counter("blight")
                } else {
                    viper.lki_counters.iter().filter(|(k, _)| k == "blight").map(|(_, n)| *n).sum()
                };
                for i in 1..=repeat {
                    for p in (0..game.seats()).filter(|p| *p != controller) {
                        let nonland: Vec<ObjId> = game
                            .battlefield
                            .iter()
                            .copied()
                            .filter(|b| game.obj(*b).controller == p && !game.has_type(*b, CardType::Land))
                            .collect();
                        if !nonland.is_empty() {
                            let q = Query::YesNo {
                                message: format!("Sacrifices a nonland permanent? (Iteration {} of {})", i, repeat),
                            };
                            let yes = loop {
                                match ask(game, d, p, &q) {
                                    Answer::Bool(b) => break b,
                                    Answer::Stop => return,
                                    _ => {}
                                }
                            };
                            if yes {
                                let Some(chosen) = choose_cards(game, d, p, &nonland, &nonland, 1, 1, "sacrifice")
                                else {
                                    return;
                                };
                                if let Some(c) = chosen.first().copied() {
                                    sacrifice(game, c);
                                    continue;
                                }
                            }
                        }
                        if !game.player(p).hand.is_empty() {
                            let q =
                                Query::YesNo { message: format!("Discard a card? (Iteration {} of {})", i, repeat) };
                            let yes = loop {
                                match ask(game, d, p, &q) {
                                    Answer::Bool(b) => break b,
                                    Answer::Stop => return,
                                    _ => {}
                                }
                            };
                            if yes {
                                discard(game, d, p, 1);
                                continue;
                            }
                        }
                        lose_life(game, p, 4);
                    }
                }
            }
            EffectDef::BloodthirstyAdversaryEffect { cost } => {
                // `DoIfAnyNumberCostPaid`: asked again after each payment (mana always counts as payable)
                let Ok(mana) = ManaCost::parse(cost) else { continue };
                let mut times = 0;
                loop {
                    let q = Query::YesNo {
                        message: format!(
                            "Pay {}? You have paid this cost {} time{}",
                            cost,
                            times,
                            if times != 1 { "s" } else { "" }
                        ),
                    };
                    let pay = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    if !pay {
                        break;
                    }
                    // (a cancelled payment isn't undone: its sources stay tapped, unlike `DoIfCostPaid`)
                    if !pay_mana(game, d, controller, &mana) {
                        if d.stopped() {
                            return;
                        }
                        break;
                    }
                    times += 1;
                }
                if times > 0 {
                    let (src, zcc) = source;
                    let (counters, target): (EffectDef, TargetDef) = (
                        serde_json::from_value(serde_json::json!({
                            "class": "AddCountersSourceEffect", "counter": {"name": "+1/+1", "amount": times}
                        }))
                        .unwrap(),
                        serde_json::from_value(serde_json::json!({
                            "class": "TargetCardInGraveyard", "min": 0, "max": times,
                            "filter": [{"p": "or", "xs": [{"p": "type", "t": "INSTANT"}, {"p": "type", "t": "SORCERY"}]},
                                       {"p": "mv", "cmp": "FEWER_THAN", "n": 4}, {"p": "owner", "who": "you"}]
                        }))
                        .unwrap(),
                    );
                    let delayed = Delayed {
                        rule: "Put that many +1/+1 counters on {this}, then exile up to that many target instant and/or sorcery \
                               cards with mana value 3 or less from your graveyard and copy them. You may cast any number of the \
                               copies without paying their mana costs."
                            .into(),
                        sacrifice: Vec::new(),
                        warp_exile: Vec::new(),
                        effects: vec![counters, EffectDef::BloodthirstyAdversaryCopyEffect],
                        source_zcc: zcc,
                        fixed: Vec::new(),
                        return_tapped: Vec::new(),
                        targets: vec![target],
                        condition: None,
                    };
                    game.pending
                        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, controller) });
                }
            }
            EffectDef::BloodthirstyAdversaryCopyEffect => {
                let cards: Vec<ObjId> = objs.iter().copied().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
                for c in &cards {
                    game.move_to(*c, Zone::Exile);
                }
                for c in cards {
                    // `game.copyCard`: a copy that leaves no card behind once it's cast and gone
                    let def = game.obj(c).def.clone();
                    let copy = game.create_obj(def.clone(), controller, Zone::Exile);
                    Arc::make_mut(&mut game.exile).push(copy);
                    game.obj_mut(copy).token = true;
                    let q = Query::YesNo { message: format!("Cast {} for free?", def.name) };
                    let yes = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    if yes {
                        cast_free(game, d, controller, copy);
                    }
                }
            }
            EffectDef::AvenInterrupterEffect => {
                // `PlotAbility.doExileAndPlotCard`: the spell is exiled, plotted (a copy just leaves)
                for o in &objs {
                    let Some(k) =
                        game.stack.iter().position(|i| i.card == *o && i.ability.is_none() && i.delayed.is_none())
                    else {
                        continue;
                    };
                    Arc::make_mut(&mut game.stack).remove(k);
                    game.move_to(*o, Zone::Exile);
                    if game.obj(*o).zone == Zone::Exile {
                        let turn = game.turn;
                        game.obj_mut(*o).plotted_turn = Some(turn);
                    }
                }
            }
            EffectDef::SpellQuellerLeavesEffect => {
                // the card exiled while it was on the battlefield (the zone change count before it left)
                let (src, zcc) = source;
                let Some(z) =
                    game.exiled_for_source.iter().filter(|(s, z, _)| *s == src && *z < zcc).map(|(_, z, _)| *z).max()
                else {
                    continue;
                };
                let Some(card) = game
                    .exiled_for_source
                    .iter()
                    .find(|(s, x, c)| *s == src && *x == z && game.obj(*c).zone == Zone::Exile)
                    .map(|(_, _, c)| *c)
                else {
                    continue;
                };
                let owner = game.obj(card).owner;
                let q = Query::YesNo { message: format!("Cast {} without paying cost?", game.obj(card).def.name) };
                let yes = loop {
                    match ask(game, d, owner, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if yes {
                    cast_free(game, d, owner, card);
                    if d.stopped() {
                        return;
                    }
                }
            }
            EffectDef::SetClassLevelEffect { level } => {
                let (me, zcc) = source;
                if game.obj(me).zone != Zone::Battlefield
                    || game.obj(me).zcc != zcc
                    || game.obj(me).class_level + 1 != *level
                {
                    continue;
                }
                game.obj_mut(me).class_level = *level;
                // `GainClassAbilitySourceEffect` adds the ability again each time continuous effects apply: it's
                // registered after every object's own
                let key = (1 << 40) + game.objs.len() as u64;
                game.obj_mut(me).class_gained.push((*level, key));
                let sources = game.trigger_sources();
                let fired = game.triggers(
                    sources,
                    |_, src, _, t| matches!(t, Trigger::ClassLevel { n } if src == me && n == level),
                );
                game.pending.extend(fired);
            }
            EffectDef::DiscardAndDrawThatManyEffect { amount } => {
                // `PlayerImpl.getToDiscard(0, amount)`: chosen from a hand that isn't empty
                let hand = game.player(controller).hand.clone();
                if hand.is_empty() || *amount < 1 {
                    return;
                }
                let Some(chosen) = choose_cards(game, d, controller, &hand, &hand, 0, *amount as usize, "discard")
                else {
                    return;
                };
                for c in &chosen {
                    game.move_to(*c, Zone::Graveyard);
                }
                draw(game, controller, chosen.len());
            }
            EffectDef::CreateTokenCopySourceEffect => {
                token_copy(game, d, source.0, controller, None);
            }
            EffectDef::CreateTokenCopyTargetEffect => {
                // `getPermanentOrLKIBattlefield`: a target that left as the spell resolved is copied as it last was
                // (Supplant Form returns it to hand first)
                for o in &objs {
                    token_copy(game, d, *o, controller, None);
                }
            }
            EffectDef::GhostVacuumEffect => {
                let (src, zcc) = source;
                let cards: Vec<ObjId> = game
                    .exiled_for_source
                    .iter()
                    .filter(|(s, z, c)| {
                        *s == src
                            && *z == zcc
                            && game.obj(*c).zone == Zone::Exile
                            && game.obj(*c).is(CardType::Creature)
                    })
                    .map(|(_, _, c)| *c)
                    .collect();
                for c in cards {
                    enter_battlefield(game, d, c, controller, false);
                    if game.obj(c).zone == Zone::Battlefield {
                        let o = game.obj_mut(c);
                        o.add_counters("flying", 1);
                        o.base_pt = Some((1, 1));
                        let mut types = o.def.subtypes.clone();
                        types.push("Spirit".into());
                        o.creature_types = Some(types);
                    }
                }
            }
            EffectDef::MightformHarmonizerEffect => {
                for o in &objs {
                    let p = game.power_toughness(*o).0;
                    if p != 0 {
                        boost(game, *o, p, 0, Duration::EndOfTurn);
                    }
                }
            }
            EffectDef::TransformSourceEffect => {
                let (src, zcc) = source;
                // `checkTransformCount`: not if it transformed since this ability was activated or triggered (the
                // ability keeps the face it came from)
                let same_face = item.def.as_ref().is_none_or(|f| f.name == game.obj(src).def.name);
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc && same_face {
                    transform(game, src);
                }
            }
            EffectDef::DamageTargetSourcePowerEffect => {
                if let Some(t) = target {
                    let power = item.lki_power.unwrap_or_else(|| game.power_toughness(source.0).0).max(0);
                    deal_damage(game, source.0, t, power, false);
                }
            }
            EffectDef::ConditionalOneShotEffect { effect, otherwise, condition, also } => {
                let holds = condition_with_kicker(game, condition, controller, source.0, item.kicked);
                if let Some(e) = if holds { effect } else { otherwise } {
                    apply_effects(game, d, item, std::slice::from_ref(&**e), legal, source);
                }
                if holds {
                    apply_effects(game, d, item, also, legal, source);
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
            EffectDef::DoIfCostPaid { effect, cost, also, otherwise, optional } => {
                // a cost that can't be paid isn't asked about (mana always counts as payable)
                let others: Vec<&CostDef> = cost.iter().filter(|c| !matches!(c, CostDef::ManaCost { .. })).collect();
                let mut paid = false;
                if others.iter().all(|c| can_pay_cost(game, controller, source.0, c)) {
                    let q = Query::YesNo { message: format!("Pay {}?", ability_mana_cost(cost).text) };
                    let pay = !*optional
                        || loop {
                            match ask(game, d, controller, &q) {
                                Answer::Bool(b) => break b,
                                Answer::Stop => return,
                                _ => {}
                            }
                        };
                    if pay {
                        let before = game.clone();
                        if others.iter().all(|c| pay_cost(game, d, controller, source.0, c))
                            && pay_mana(game, d, controller, &ability_mana_cost(cost))
                        {
                            paid = true;
                            if let Some(effect) = effect {
                                apply_effects(game, d, item, std::slice::from_ref(&**effect), legal, source);
                            }
                            apply_effects(game, d, item, also, legal, source);
                        } else if d.stopped() {
                            return;
                        } else {
                            // `DoIfCostPaid` undoes a payment that was cancelled
                            *game = before;
                        }
                    }
                }
                if !paid {
                    apply_effects(game, d, item, otherwise, legal, source);
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
            EffectDef::GainAbilityControlledEffect {
                ability,
                duration: Duration::UntilYourNextTurn,
                filter,
                exclude_source,
            } => {
                for id in Arc::clone(&game.battlefield).iter().copied() {
                    if game.obj(id).controller == controller
                        && game.filter_matches(filter, controller, Some(source.0), id)
                        && !(*exclude_source && id == source.0)
                    {
                        let zcc = game.obj(id).zcc;
                        Arc::make_mut(&mut game.grants).push(Grant {
                            target: id,
                            zcc,
                            ability: (**ability).clone(),
                            until_end_of_turn: false,
                            until_turn_of: Some(controller),
                        });
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
            EffectDef::GainAbilityTargetEffect { ability, duration } => {
                for o in &objs {
                    // an activated ability for good (Quicksmith Spy's "{T}: Draw a card"): one of the permanent's own,
                    // after its printed ones
                    if *duration == Duration::WhileOnBattlefield
                        && ability.activated().is_some()
                        && game.obj(*o).zone == Zone::Battlefield
                    {
                        let p = game.obj_mut(*o);
                        let mut def = (*p.def).clone();
                        def.abilities.push((**ability).clone());
                        if p.printed_def.is_none() {
                            p.printed_def = Some(p.def.clone());
                        }
                        p.def = Arc::new(def);
                        continue;
                    }
                    grant(game, *o, ability);
                    // "for as long as you control …" (taken as for good)
                    if *duration == Duration::WhileOnBattlefield {
                        if let Some(g) = Arc::make_mut(&mut game.grants).last_mut() {
                            g.until_end_of_turn = false;
                        }
                    }
                    if **ability == AbilityDef::BlocksIfAbleAbility {
                        let zcc = game.obj(*o).zcc;
                        game.must_block_from.push((*o, zcc, source.0));
                    }
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
            EffectDef::DamageAllControlledTargetEffect { amount, filter } => {
                let amount = &v!(amount);
                let who = match target {
                    Some(Target::Player(p)) => p,
                    Some(Target::Obj(o)) => game.obj(o).controller,
                    None => continue,
                };
                for o in matching(game, filter, controller, source.0) {
                    if game.obj(o).controller == who {
                        deal_damage(game, source.0, Target::Obj(o), *amount, false);
                    }
                }
            }
            EffectDef::ReturnFromYourGraveyardToBattlefieldAllEffect { filter } => {
                let cards: Vec<ObjId> = game
                    .player(controller)
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|c| game.filter_matches(filter, controller, Some(source.0), *c))
                    .collect();
                for c in cards {
                    enter_battlefield(game, d, c, controller, false);
                }
            }
            EffectDef::DamageAllEffect { amount, filter } => {
                let amount = &v!(amount);
                for o in matching(game, filter, controller, source.0) {
                    deal_damage(game, source.0, Target::Obj(o), *amount, false);
                }
            }
            EffectDef::BoostAllEffect { power, toughness, duration: Duration::EndOfTurn, filter, exclude_source } => {
                let power = &v!(power);
                let toughness = &v!(toughness);
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
                        put_counters(game, o, &counter.name, n, controller);
                    }
                }
            }
            // `FightTargetsEffect`: only with both targets legal; each is dealt the other's power, the first first
            EffectDef::FightTargetsEffect => {
                if let (Some(Target::Obj(a)), Some(Target::Obj(b))) = (target, legal_target(1)) {
                    let creature =
                        |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
                    if creature(game, a) && creature(game, b) {
                        // both powers as they were: -1/-1 counters put on during the fight (Soul-Scar Mage) don't
                        // count until effects are applied again
                        let pb = game.power_toughness(b).0;
                        let pa = game.power_toughness(a).0;
                        deal_damage(game, b, Target::Obj(a), pb, false);
                        // `fightWithExcess`: the second creature is dealt only lethal damage (`damageWithExcess`)
                        let pa = pa.min(fight_lethal(game, b, a));
                        deal_damage(game, a, Target::Obj(b), pa, false);
                    }
                }
            }
            // `FightEnchantedTargetEffect`: the Aura's creature (last known) fights the target
            EffectDef::FightEnchantedTargetEffect => {
                let enchanted = game.obj(source.0).attached_to.and_then(|(o, zcc)| {
                    (game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc).then_some(o)
                });
                if let (Some(a), Some(Target::Obj(b))) = (enchanted, target) {
                    if game.obj(b).zone == Zone::Battlefield && game.has_type(b, CardType::Creature) {
                        let pb = game.power_toughness(b).0;
                        let pa = game.power_toughness(a).0;
                        deal_damage(game, b, Target::Obj(a), pb, false);
                        let pa = pa.min(fight_lethal(game, b, a));
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
                        let mv = game.obj(o).def.mana_value() as i32;
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
                        game.has_type(*o, CardType::Creature) && (game.obj(*o).def.mana_value() % 2 == 1) == odd
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
                // the source's zone for them: a leaves trigger's is the permanent's that left (its stack moment's)
                let zone = if item.ability.is_some() && game.obj(source.0).zone != Zone::Battlefield {
                    (source.0, source.1.saturating_sub(1))
                } else {
                    source
                };
                for c in &cards {
                    game.move_to(*c, Zone::Exile);
                    if game.obj(*c).zone == Zone::Exile {
                        game.obj_mut(*c).exiled_with = Some(zone);
                    }
                }
                let mut cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
                if *choose_one && cards.len() > 1 {
                    let rule_text = item_rule(game, item);
                    let Some(chosen) = choose_cards(game, d, controller, &cards, &cards, 1, 1, &rule_text) else {
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
            // asked only when the cost can be paid; a cancelled payment is undone
            EffectDef::DoWhenCostPaid { cost, message, rule, targets, effects } => {
                let others: Vec<&CostDef> = cost.iter().filter(|c| !matches!(c, CostDef::ManaCost { .. })).collect();
                if !others.iter().all(|c| can_pay_cost(game, controller, source.0, c)) {
                    return;
                }
                let q = Query::YesNo { message: message.clone() };
                let pay = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if pay {
                    let before = game.clone();
                    if others.iter().all(|c| pay_cost(game, d, controller, source.0, c))
                        && pay_mana(game, d, controller, &ability_mana_cost(cost))
                    {
                        let delayed = Delayed {
                            rule: rule.clone(),
                            sacrifice: Vec::new(),
                            warp_exile: Vec::new(),
                            effects: effects.clone(),
                            source_zcc: source.1,
                            fixed: Vec::new(),
                            return_tapped: Vec::new(),
                            targets: targets.clone(),
                            condition: None,
                        };
                        game.pending.push(StackItem {
                            delayed: Some(Arc::new(delayed)),
                            ..StackItem::spell(source.0, controller)
                        });
                    } else if d.stopped() {
                        return;
                    } else {
                        *game = before;
                    }
                }
            }
            EffectDef::AddCounterSourceReflexiveEffect { counter, rule, condition, targets, effects } => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    game.obj_mut(src).add_counters(&counter.name, counter.amount);
                    let delayed = Delayed {
                        rule: rule.clone(),
                        sacrifice: Vec::new(),
                        warp_exile: Vec::new(),
                        effects: effects.clone(),
                        source_zcc: zcc,
                        fixed: Vec::new(),
                        return_tapped: Vec::new(),
                        targets: targets.clone(),
                        condition: condition.clone(),
                    };
                    game.pending
                        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, controller) });
                }
            }
            EffectDef::PinnacleStarcageExileEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    let filter = crate::card::Filter(vec![
                        Pred::Or {
                            xs: vec![Pred::Type { t: CardType::Artifact }, Pred::Type { t: CardType::Creature }],
                        },
                        Pred::Mv { cmp: Cmp::FewerThan, n: 3 },
                    ]);
                    for o in matching(game, &filter, controller, src) {
                        game.move_to(o, Zone::Exile);
                        game.exiled_until_leaves.push((src, zcc, o));
                    }
                }
            }
            EffectDef::PinnacleStarcageTokenEffect { token } => {
                let (src, zcc) = source;
                let exiled: Vec<ObjId> = game
                    .exiled_until_leaves
                    .iter()
                    .filter(|(s, z, c)| {
                        *s == src && *z == zcc && game.obj(*c).zone == Zone::Exile && !game.obj(*c).token
                    })
                    .map(|(_, _, c)| *c)
                    .collect();
                for c in &exiled {
                    game.move_to(*c, Zone::Graveyard);
                }
                let n = exiled.iter().filter(|c| game.obj(**c).zone == Zone::Graveyard).count() as i32;
                if n > 0 {
                    let e = EffectDef::CreateTokenEffect {
                        token: token.clone(),
                        amount: Amount::Fixed(n),
                        tapped_and_attacking: false,
                        tapped: false,
                        attacking: false,
                        sacrifice_at_end_of_combat: false,
                        sacrifice_at_next_end_step: false,
                        haste_until_end_of_turn: false,
                        also: None,
                        attach_source: None,
                    };
                    apply_effects(game, d, item, std::slice::from_ref(&e), &[], source);
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
            EffectDef::ReturnSourceFromGraveyardToBattlefieldEffect { tapped } => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Graveyard && game.obj(o).zcc == zcc {
                    enter_battlefield(game, d, o, controller, false);
                    if *tapped {
                        game.obj_mut(o).tapped = true;
                    }
                }
            }
            EffectDef::DamageCantBePreventedEffect { .. } => game.no_prevention_turn = Some(game.turn),
            EffectDef::CantGainLifeThisTurnEffect => game.no_lifegain_turn = Some(game.turn),
            EffectDef::CastAsThoughItHadFlashUntilYourNextTurnEffect { filter } => {
                game.flash_until_next_turn.push((controller, filter.clone(), source.0))
            }
            EffectDef::CantCastMoreThanOneSpellEffect { .. }
            | EffectDef::CantCastDuringYourTurnEffect
            | EffectDef::ReduceCostEquipTargetSourceEffect { .. }
            | EffectDef::PermanentsEnterBattlefieldTappedEffect { .. }
            | EffectDef::CantGainLifeAllEffect
            | EffectDef::AttacksIfAbleSourceEffect { .. }
            | EffectDef::DontUntapInControllersUntapStepSourceEffect
            | EffectDef::ClarionConquerorEffect
            | EffectDef::CastAsThoughItHadFlashAllEffect { .. }
            | EffectDef::NarsetParterOfVeilsEffect
            | EffectDef::TeferiTimeRavelerEffect
            | EffectDef::NoMaximumHandSizeEffect
            | EffectDef::CreateTwiceThatManyTokensEffect
            | EffectDef::PlayAdditionalLandsControllerEffect { .. }
            | EffectDef::PlayLandsFromGraveyardControllerEffect
            | EffectDef::GevScaledScorchEntersEffect
            | EffectDef::LovestruckBeastEffect
            | EffectDef::GainPlusOneLifeReplacementEffect
            | EffectDef::SpellCostReductionForEachSourceEffect { .. }
            | EffectDef::CantBeCounteredControlledEffect { .. }
            | EffectDef::DontCauseTriggerEffect
            | EffectDef::SetBasePowerToughnessPlusOneSourceEffect { .. }
            | EffectDef::SpellsCostIncreasingAllEffect { .. }
            | EffectDef::SpellsCostReductionControllerEffect { .. }
            | EffectDef::WeatheredRunestoneEffect
            | EffectDef::WeatheredRunestoneEffect2
            | EffectDef::ChooseCreatureEffect { .. }
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
                    let mv = game.obj(o).def.mana_value() as i32;
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
            EffectDef::MayCastTargetFromGraveyardThisTurnEffect => {
                let turn = game.turn;
                for o in &objs {
                    if game.obj(*o).zone == Zone::Graveyard && !game.obj(*o).is(CardType::Land) {
                        game.obj_mut(*o).may_play = Some((controller, turn));
                    }
                }
            }
            EffectDef::LoseAbilityAllEffect { ability, filter, .. } => {
                let turn = game.turn;
                let lose: Vec<(ObjId, u32, AbilityDef, u32)> = game
                    .battlefield
                    .iter()
                    .filter(|p| game.filter_matches(filter, controller, Some(source.0), **p))
                    .map(|p| (*p, game.obj(*p).zcc, (**ability).clone(), turn))
                    .collect();
                game.lost_until_eot.retain(|(_, _, _, t)| *t == turn);
                game.lost_until_eot.extend(lose);
            }
            EffectDef::GainAbilityPermanentlyEffect { ability } => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    let p = game.obj_mut(o);
                    let mut def = (*p.def).clone();
                    def.abilities.push((**ability).clone());
                    if p.printed_def.is_none() {
                        p.printed_def = Some(p.def.clone());
                    }
                    p.def = Arc::new(def);
                }
            }
            EffectDef::PrismaticEndingEffect => {
                let colors = game.obj(source.0).spent_colors.iter().filter(|n| **n > 0).count() as u32;
                if let Some(o) = objs.first().copied().filter(|o| game.obj(*o).zone == Zone::Battlefield) {
                    if game.obj(o).def.mana_value() <= colors {
                        game.move_to(o, Zone::Exile);
                    }
                }
            }
            EffectDef::RelicOfProgenitusEffect => {
                let Some(p) = item.targets.iter().flatten().find_map(|t| match t {
                    Target::Player(p) => Some(*p),
                    _ => None,
                }) else {
                    continue;
                };
                let cards = game.player(p).graveyard.clone();
                // `canChoose` first: an empty graveyard asks nothing
                if cards.is_empty() {
                    continue;
                }
                let Some(chosen) = choose_cards(game, d, p, &cards, &cards, 1, 1, "exiles") else { return };
                for c in chosen {
                    game.move_to(c, Zone::Exile);
                }
            }
            EffectDef::ExileAndGainLifeEqualPowerTargetEffect => {
                if let Some(o) = objs.first().copied().filter(|o| game.obj(*o).zone == Zone::Battlefield) {
                    let (power, who) = (game.power_toughness(o).0, game.obj(o).controller);
                    game.move_to(o, Zone::Exile);
                    gain_life(game, who, power);
                }
            }
            EffectDef::ExileThenReturnTargetEffect => {
                let gone: Vec<ObjId> =
                    objs.iter().copied().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
                for o in &gone {
                    game.move_to(*o, Zone::Exile);
                }
                for o in gone {
                    // (a token ceases to exist in exile)
                    if game.obj(o).zone == Zone::Exile && !game.obj(o).token {
                        let owner = game.obj(o).owner;
                        enter_battlefield(game, d, o, owner, false);
                    }
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
                    tap(game, *o);
                }
            }
            EffectDef::EdificeOfAuthorityEffect => {
                for o in &objs {
                    let zcc = game.obj(*o).zcc;
                    for ability in [
                        AbilityDef::CantAttackAbility,
                        AbilityDef::CantBlockAbility,
                        AbilityDef::CantActivateAbilitiesAbility,
                    ] {
                        Arc::make_mut(&mut game.grants).push(Grant {
                            target: *o,
                            zcc,
                            ability,
                            until_end_of_turn: false,
                            until_turn_of: Some(controller),
                        });
                    }
                }
            }
            EffectDef::BelieveEffect => {
                if let Some(c) = game.player(controller).library.first().copied() {
                    let put = game.obj(c).is(CardType::Creature) && {
                        let r = crate::replay::reference(game, c);
                        let q =
                            Query::YesNo { message: format!("Put {} [{r}] onto the battlefield?", game.obj(c).name()) };
                        ask(game, d, controller, &q) == Answer::Bool(true)
                    };
                    if put {
                        enter_battlefield(game, d, c, controller, false);
                    } else {
                        game.move_to(c, Zone::Hand);
                    }
                }
            }
            EffectDef::NissasEncouragementEffect => {
                // (`HashMap` order of the names)
                let names = ["Brambleweft Behemoth", "Forest", "Nissa, Genesis Mage"];
                let filter = crate::card::Filter(vec![Pred::Or {
                    xs: names.iter().map(|n| Pred::Name { n: n.to_string() }).collect(),
                }]);
                let Some(found) = search_library(game, d, controller, &filter, 0, 3, source.0) else { return };
                // one card of each name
                let mut cards: Vec<ObjId> = Vec::new();
                for c in found {
                    if !cards.iter().any(|x| game.obj(*x).name() == game.obj(c).name()) {
                        cards.push(c);
                    }
                }
                if cards.len() < 3 {
                    for n in names {
                        if cards.iter().any(|x| game.obj(*x).name() == n) {
                            continue;
                        }
                        let grave = game.player(controller).graveyard.clone();
                        let possible: Vec<ObjId> = grave.iter().copied().filter(|c| game.obj(*c).name() == n).collect();
                        if possible.is_empty() {
                            continue;
                        }
                        let rule_text = item_rule(game, item);
                        let Some(chosen) = choose_cards(game, d, controller, &grave, &possible, 0, 1, &rule_text)
                        else {
                            return;
                        };
                        cards.extend(chosen);
                    }
                }
                if !cards.is_empty() {
                    let zcc = game.obj(source.0).zcc;
                    let revealed = cards.iter().map(|c| (*c, game.obj(*c).zone)).collect();
                    game.revealed.push(crate::state::Revealed { source: source.0, zcc, paren: false, cards: revealed });
                    for c in cards {
                        game.move_to(c, Zone::Hand);
                    }
                }
                let mut library = game.player(controller).library.clone();
                game.rng.shuffle(&mut library);
                game.player_mut(controller).library = library;
            }
            EffectDef::BountyOfTheLuxaEffect => {
                let (id, zcc) = source;
                let here = game.obj(id).zone == Zone::Battlefield && game.obj(id).zcc == zcc;
                let flood = if here { game.obj(id).counter("flood") } else { 0 };
                if flood > 0 {
                    game.obj_mut(id).counters.retain(|(n, _)| n != "flood");
                    let pool = &mut game.player_mut(controller).mana_pool;
                    pool.colorless += 1;
                    pool.colored[Color::G.index()] += 1;
                    pool.colored[Color::U.index()] += 1;
                } else {
                    if here {
                        game.obj_mut(id).add_counters("flood", 1);
                    }
                    draw(game, controller, 1);
                }
            }
            EffectDef::GoryosVengeanceEffect => {
                for o in &objs {
                    if game.obj(*o).zone != Zone::Graveyard {
                        continue;
                    }
                    enter_battlefield(game, d, *o, controller, false);
                    if game.obj(*o).zone != Zone::Battlefield {
                        continue;
                    }
                    let zcc = game.obj(*o).zcc;
                    // haste for as long as it's this permanent (`Duration.Custom`)
                    Arc::make_mut(&mut game.grants).push(Grant {
                        target: *o,
                        zcc,
                        ability: AbilityDef::HasteAbility,
                        until_end_of_turn: false,
                        until_turn_of: None,
                    });
                    let name = game.obj(*o).def.name.clone();
                    let delayed = Delayed {
                        rule: format!(
                            "At the beginning of the next end step, Exile {name} at the beginning of the next end step."
                        ),
                        sacrifice: Vec::new(),
                        warp_exile: Vec::new(),
                        effects: vec![EffectDef::ExileTargetEffect],
                        source_zcc: source.1,
                        fixed: vec![(*o, zcc)],
                        return_tapped: Vec::new(),
                        targets: Vec::new(),
                        condition: None,
                    };
                    game.at_next_end_step
                        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
                }
            }
            EffectDef::AtraxaGrandUnifierEffect => {
                let cards: Vec<ObjId> = game.player(controller).library.iter().take(10).copied().collect();
                if cards.is_empty() {
                    continue;
                }
                let zcc = game.obj(source.0).zcc;
                let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
                game.revealed.push(crate::state::Revealed { source: source.0, zcc, paren: false, cards: shown });
                // `AtraxaGrandUnifierTarget`: a card can join the chosen ones while each still gets a card type of
                // its own (`CardTypeAssignment`); `HumanPlayer.choose` until done or nothing more fits
                let mut chosen: Vec<ObjId> = Vec::new();
                loop {
                    let possible: Vec<ObjId> = cards
                        .iter()
                        .copied()
                        .filter(|c| {
                            !chosen.contains(c) && {
                                let mut with = chosen.clone();
                                with.push(*c);
                                distinct_card_types(game, &with)
                            }
                        })
                        .collect();
                    match ask(game, d, controller, &Query::Cards { objs: possible.clone(), required: false }) {
                        Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                        Answer::Obj(o) if possible.contains(&o) => {
                            chosen.push(o);
                            let more = cards.iter().any(|c| {
                                !chosen.contains(c) && {
                                    let mut with = chosen.clone();
                                    with.push(*c);
                                    distinct_card_types(game, &with)
                                }
                            });
                            if !more {
                                break;
                            }
                        }
                        Answer::Stop => return,
                        Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                        _ => break,
                    }
                    if d.stopped() {
                        return;
                    }
                }
                for c in &chosen {
                    game.move_to(*c, Zone::Hand);
                }
                let rest: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).zone == Zone::Library).collect();
                if !put_on_library(game, d, controller, &rest, false, false) {
                    return;
                }
            }
            EffectDef::SacrificeTargetNextEndStepEffect => {
                if let Some(Target::Obj(o)) = target {
                    if game.obj(o).zone == Zone::Battlefield {
                        let delayed = Delayed {
                            rule: format!(
                                "At the beginning of the next end step, Sacrifice boosted {}.",
                                game.obj(o).def.name
                            ),
                            sacrifice: vec![(o, game.obj(o).zcc)],
                            warp_exile: Vec::new(),
                            effects: Vec::new(),
                            source_zcc: source.1,
                            fixed: Vec::new(),
                            return_tapped: Vec::new(),
                            targets: Vec::new(),
                            condition: None,
                        };
                        game.at_next_end_step.push(StackItem {
                            delayed: Some(Arc::new(delayed)),
                            ..StackItem::spell(source.0, controller)
                        });
                    }
                }
            }
            EffectDef::AidFromTheCowlEffect => {
                let Some(card) = game.player(controller).library.first().copied() else { continue };
                let name = log::object(game, card);
                let permanent = game.obj(card).def.is_permanent_card();
                let yes = |game: &mut Game, d: &mut dyn Decider, message: String| loop {
                    match ask(game, d, controller, &Query::YesNo { message: message.clone() }) {
                        Answer::Bool(b) => break Some(b),
                        Answer::Stop => break None,
                        _ => {}
                    }
                };
                let put = if permanent {
                    match yes(game, d, format!("Put {name} onto the battlefield?")) {
                        Some(b) => b,
                        None => return,
                    }
                } else {
                    false
                };
                if put {
                    enter_battlefield(game, d, card, controller, false);
                } else {
                    match yes(game, d, format!("Put {name} on the bottom of your library?")) {
                        Some(true) => to_library(game, card, false),
                        Some(false) => {}
                        None => return,
                    }
                }
            }
            EffectDef::TreasureKeeperEffect => {
                let mut revealed = Vec::new();
                let mut hit = None;
                for c in game.player(controller).library.clone() {
                    revealed.push(c);
                    let def = &game.obj(c).def;
                    if !def.is(CardType::Land) && def.mana_value() < 4 {
                        hit = Some(c);
                        break;
                    }
                }
                if let Some(c) = hit {
                    let message = format!("Cast {} without paying its mana cost?", log::object(game, c));
                    let yes = loop {
                        match ask(game, d, controller, &Query::YesNo { message: message.clone() }) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    if yes && cast_free(game, d, controller, c) {
                        revealed.retain(|x| *x != c);
                    }
                    if d.stopped() {
                        return;
                    }
                }
                let revealed: Vec<ObjId> =
                    revealed.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
                if !put_on_library(game, d, controller, &revealed, false, false) {
                    return;
                }
            }
            EffectDef::PreventCombatDamageByChosenEffect { filter } => {
                let can = matching(game, filter, controller, source.0);
                if can.is_empty() {
                    continue;
                }
                let Some(chosen) = choose_cards(game, d, controller, &can, &can, 1, 1, "creature of your choice")
                else {
                    return;
                };
                let turn = game.turn;
                game.prevent_combat_from.retain(|(_, _, t)| *t == turn);
                for c in chosen {
                    let zcc = game.obj(c).zcc;
                    game.prevent_combat_from.push((c, zcc, turn));
                }
            }
            EffectDef::PreventAllDamageToSourceEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield {
                    let turn = game.turn;
                    game.prevent_all_to.retain(|(_, _, t)| *t == turn);
                    game.prevent_all_to.push((src, zcc.max(game.obj(src).zcc), turn));
                }
            }
            EffectDef::TapAllEffect { filter } => {
                for o in matching(game, filter, controller, source.0) {
                    if !game.obj(o).tapped {
                        tap(game, o);
                    }
                }
            }
            EffectDef::RevealUntilToHandEffect { filter } => {
                let mut revealed = Vec::new();
                let mut hit = None;
                for c in game.player(controller).library.clone() {
                    if game.filter_matches(filter, controller, Some(source.0), c) {
                        hit = Some(c);
                        break;
                    }
                    revealed.push(c);
                }
                if let Some(c) = hit {
                    game.move_to(c, Zone::Hand);
                }
                if !put_on_library(game, d, controller, &revealed, false, false) {
                    return;
                }
            }
            EffectDef::GainControlAllControlledTargetEffect { filter } => {
                if let Some(Target::Player(p)) = target {
                    for o in matching(game, filter, controller, source.0) {
                        if game.obj(o).controller == p {
                            let ob = game.obj_mut(o);
                            ob.controller = controller;
                            ob.sick = true;
                        }
                    }
                }
            }
            EffectDef::MerchantsDockhandEffect => {
                let n = item.x.max(0) as usize;
                let cards: Vec<ObjId> = game.player(controller).library.iter().take(n).copied().collect();
                if cards.is_empty() {
                    continue;
                }
                let Some(chosen) =
                    choose_cards(game, d, controller, &cards, &cards, 1, 1, "card to put into your hand")
                else {
                    return;
                };
                for c in &chosen {
                    game.move_to(*c, Zone::Hand);
                }
                let rest: Vec<ObjId> = cards.into_iter().filter(|c| !chosen.contains(c)).collect();
                if !put_on_library(game, d, controller, &rest, false, true) {
                    return;
                }
            }
            EffectDef::OraclesVaultFreeEffect => {
                if let Some(c) = game.player(controller).library.first().copied() {
                    game.move_to(c, Zone::Exile);
                    if game.obj(c).zone == Zone::Exile {
                        let turn = game.turn;
                        let o = game.obj_mut(c);
                        o.may_play = Some((controller, turn));
                        o.play_free = true;
                    }
                }
            }
            EffectDef::NissaNaturesArtisanEffect => {
                let top: Vec<ObjId> = game.player(controller).library.iter().take(2).copied().collect();
                let (lands, rest): (Vec<ObjId>, Vec<ObjId>) =
                    top.into_iter().partition(|c| game.obj(*c).def.is(CardType::Land));
                for c in lands {
                    enter_battlefield(game, d, c, controller, false);
                }
                for c in rest {
                    game.move_to(c, Zone::Hand);
                }
            }
            EffectDef::TapEnchantedEffect => {
                if let Some(o) = game.attached(source.0) {
                    tap(game, o);
                }
            }
            EffectDef::DiscardHandDrawSameNumberSourceEffect => {
                let hand = game.player(controller).hand.clone();
                for c in &hand {
                    game.move_to(*c, Zone::Graveyard);
                }
                draw(game, controller, hand.len());
            }
            EffectDef::UntapAllEffect { filter } => {
                for o in matching(game, filter, controller, source.0) {
                    game.obj_mut(o).tapped = false;
                }
            }
            EffectDef::MayTapOrUntapTargetEffect => {
                if let Some(o) = objs.first().copied() {
                    let tapped = game.obj(o).tapped;
                    let message = if tapped { "Untap that permanent?" } else { "Tap that permanent?" };
                    match ask(game, d, controller, &Query::YesNo { message: message.into() }) {
                        Answer::Bool(true) => game.obj_mut(o).tapped = !tapped,
                        Answer::Stop => return,
                        _ => {}
                    }
                }
            }
            EffectDef::ReturnToHandSourceEffect { battlefield_only } => {
                let (o, zcc) = source;
                let z = game.obj(o).zone;
                if game.obj(o).zcc == zcc
                    && (z == Zone::Battlefield || !*battlefield_only && matches!(z, Zone::Graveyard | Zone::Exile))
                {
                    game.move_to(o, Zone::Hand);
                }
            }
            EffectDef::UntapSourceEffect => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    untap(game.obj_mut(o));
                }
            }
            EffectDef::RegenerateTargetEffect => {
                for o in &objs {
                    if game.obj(*o).zone == Zone::Battlefield {
                        game.obj_mut(*o).regen_shields += 1;
                    }
                }
            }
            EffectDef::LoseAllAbilitiesTargetEffect => {
                for o in &objs {
                    if game.obj(*o).zone != Zone::Battlefield {
                        continue;
                    }
                    let ob = game.obj_mut(*o);
                    let old = ob.def.clone();
                    let mut def = (*old).clone();
                    def.abilities.clear();
                    ob.def = Arc::new(def);
                    if ob.until_end_of_turn_def.is_none() {
                        ob.until_end_of_turn_def = Some(old);
                    }
                }
            }
            EffectDef::UntapTargetEffect => {
                for o in &objs {
                    untap(game.obj_mut(*o));
                }
            }
            EffectDef::ReturnToHandEachTargetEffect => {
                for t in legal.iter().flatten() {
                    if let Target::Obj(o) = t {
                        if game.obj(*o).zone == Zone::Stack {
                            Arc::make_mut(&mut game.stack).retain(|i| i.card != *o || i.ability.is_some());
                        }
                        game.move_to(*o, Zone::Hand);
                    }
                }
            }
            EffectDef::UntapEachTargetEffect => {
                for t in legal.iter().flatten() {
                    if let Target::Obj(o) = t {
                        untap(game.obj_mut(*o));
                    }
                }
            }
            EffectDef::TapSourceEffect => {
                let (o, zcc) = source;
                if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                    tap(game, o);
                }
            }
            EffectDef::DamageControllerEffect { amount } => {
                let amount = &v!(amount);
                deal_damage(game, source.0, Target::Player(controller), *amount, false);
            }
            EffectDef::ScryEffect { amount } => {
                let amount = &v!(amount);
                if !scry(game, d, controller, *amount, &item_rule(game, item)) {
                    return;
                }
            }
            EffectDef::ScryTargetEffect { amount } => {
                let amount = &v!(amount);
                for p in &players {
                    if !scry(game, d, *p, *amount, &item_rule(game, item)) {
                        return;
                    }
                }
            }
            EffectDef::ShiftingWoodlandCopyEffect => {
                let (src, zcc) = source;
                if let Some(Target::Obj(c)) = target {
                    if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                        let copy = game.obj(c).copiable();
                        let ob = game.obj_mut(src);
                        let old = ob.def.clone();
                        if ob.until_end_of_turn_def.is_none() {
                            ob.until_end_of_turn_def = Some(old);
                        }
                        ob.def = copy;
                    }
                }
            }
            EffectDef::EmrakulThePromisedEndGainControlEffect => {
                if let Some(Target::Player(p)) = target {
                    game.control_next_turn.push((p, controller));
                }
            }
            EffectDef::SearchLibraryGraveyardPutInHandEffect { filter, both } => {
                let mut found = None;
                let mut shuffle = false;
                let q = Query::YesNo { message: "Search your library?".into() };
                if *both || ask(game, d, controller, &q) == Answer::Bool(true) {
                    let Some(cards) = search_library(game, d, controller, filter, 0, 1, source.0) else { return };
                    found = cards.first().copied();
                    shuffle = true;
                }
                if found.is_none() {
                    let q = Query::YesNo { message: "Search your graveyard?".into() };
                    if ask(game, d, controller, &q) == Answer::Bool(true) {
                        let grave = game.player(controller).graveyard.clone();
                        let possible: Vec<ObjId> = grave
                            .iter()
                            .copied()
                            .filter(|c| game.filter_matches(filter, controller, Some(source.0), *c))
                            .collect();
                        // (an empty graveyard: nothing asked; one without a match is still asked)
                        if !grave.is_empty() {
                            let rule_text = item_rule(game, item);
                            let Some(chosen) = choose_cards(game, d, controller, &grave, &possible, 0, 1, &rule_text)
                            else {
                                return;
                            };
                            found = chosen.first().copied();
                        }
                    }
                }
                if let Some(c) = found {
                    game.move_to(c, Zone::Hand);
                }
                if shuffle {
                    let mut library = game.player(controller).library.clone();
                    game.rng.shuffle(&mut library);
                    game.player_mut(controller).library = library;
                }
            }
            EffectDef::GetEnergyCountersControllerEffect { amount } => {
                let n = v!(amount);
                if n > 0 {
                    add_energy(game, controller, n);
                }
            }
            EffectDef::BladeOfTheBloodchiefEffect => {
                if let Some((c, zcc)) = game.obj(source.0).attached_to {
                    if game.obj(c).zone == Zone::Battlefield && game.obj(c).zcc == zcc {
                        let n = if game.has_subtype(c, "Vampire") { 2 } else { 1 };
                        put_counters(game, c, "+1/+1", n, controller);
                    }
                }
            }
            EffectDef::UginsLabyrinthExileEffect => {
                let hand = game.player(controller).hand.clone();
                let possible: Vec<ObjId> = hand
                    .iter()
                    .copied()
                    .filter(|c| game.colors(*c).is_empty() && game.obj(*c).def.mana_value() >= 7)
                    .collect();
                let Some(chosen) = choose_cards(game, d, controller, &hand, &possible, 0, 1, "exile") else {
                    return;
                };
                let (src, zcc) = source;
                for c in chosen {
                    game.move_to(c, Zone::Exile);
                    game.exiled_for_source.push((src, zcc, c));
                }
            }
            EffectDef::UginsLabyrinthReturnEffect => {
                let (src, zcc) = source;
                let exiled: Vec<ObjId> = game
                    .exiled_for_source
                    .iter()
                    .filter(|(s, z, c)| *s == src && *z == zcc && game.obj(*c).zone == Zone::Exile)
                    .map(|(_, _, c)| *c)
                    .collect();
                for c in exiled {
                    game.move_to(c, Zone::Hand);
                }
            }
            // "you may pay {cost} any number of times": asked until they don't (a mana cost can always be paid); then
            // the effects with X the times paid, as a reflexive trigger when it has a rule
            EffectDef::DoIfAnyNumberCostPaid { cost, effects, reflexive } => {
                let mana = ability_mana_cost(cost);
                let mut times = 0;
                loop {
                    let message = format!(
                        "Pay {}? You have paid this cost {} time{}",
                        mana.text,
                        times,
                        if times != 1 { "s" } else { "" }
                    );
                    let yes = loop {
                        match ask(game, d, controller, &Query::YesNo { message: message.clone() }) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    if !yes || !pay_mana(game, d, controller, &mana) {
                        break;
                    }
                    times += 1;
                }
                if d.stopped() {
                    return;
                }
                if times > 0 {
                    match reflexive {
                        Some(rule) => {
                            let delayed = Delayed {
                                rule: rule.clone(),
                                sacrifice: Vec::new(),
                                warp_exile: Vec::new(),
                                effects: effects.clone(),
                                source_zcc: source.1,
                                fixed: Vec::new(),
                                return_tapped: Vec::new(),
                                targets: Vec::new(),
                                condition: None,
                            };
                            game.pending.push(StackItem {
                                delayed: Some(Arc::new(delayed)),
                                x: times,
                                ..StackItem::spell(source.0, controller)
                            });
                        }
                        None => {
                            let inner = StackItem { x: times, ..item.clone() };
                            apply_effects(game, d, &inner, effects, legal, source);
                        }
                    }
                }
            }
            EffectDef::ChosenCreatureGainsAbilityEffect { ability } => {
                if let Some((o, zcc)) = game.obj(source.0).chosen {
                    if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
                        grant(game, o, ability);
                    }
                }
            }
            EffectDef::EvolveEffect => {
                let (me, zcc) = source;
                if game.obj(me).zone == Zone::Battlefield
                    && game.obj(me).zcc == zcc
                    && item.entering.is_some_and(|e| evolves(game, me, e))
                {
                    game.obj_mut(me).add_counters("+1/+1", 1);
                }
            }
            EffectDef::RegenerateSourceEffect => {
                let (me, zcc) = source;
                if game.obj(me).zone == Zone::Battlefield && game.obj(me).zcc == zcc {
                    game.obj_mut(me).regen_shields += 1;
                }
            }
            // return the Vehicle card; it gains haste, and goes back to your hand at the beginning of the next end step
            EffectDef::GreasefangOkibaBossEffect => {
                let Some(Target::Obj(c)) = target else { continue };
                if game.obj(c).zone != Zone::Graveyard {
                    continue;
                }
                enter_battlefield(game, d, c, controller, false);
                if game.obj(c).zone == Zone::Battlefield {
                    let zcc = game.obj(c).zcc;
                    Arc::make_mut(&mut game.grants).push(Grant {
                        target: c,
                        zcc,
                        ability: AbilityDef::HasteAbility,
                        until_end_of_turn: false,
                        until_turn_of: None,
                    });
                    let delayed = Delayed {
                        rule: "At the beginning of the next end step, return it to your hand.".into(),
                        sacrifice: Vec::new(),
                        warp_exile: Vec::new(),
                        effects: vec![EffectDef::ReturnToHandTargetEffect],
                        source_zcc: source.1,
                        fixed: vec![(c, zcc)],
                        return_tapped: Vec::new(),
                        targets: Vec::new(),
                        condition: None,
                    };
                    game.at_next_end_step
                        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
                }
            }
            EffectDef::GainsChoiceOfAbilitiesEffect { abilities, names } => {
                let (me, zcc) = source;
                if game.obj(me).zone != Zone::Battlefield || game.obj(me).zcc != zcc {
                    continue;
                }
                let q = Query::Choice { message: "Choose an ability to gain".into(), choices: names.clone() };
                let k = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Text(t) => {
                            if let Some(k) = names.iter().position(|n| *n == t) {
                                break k;
                            }
                        }
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                grant(game, me, &abilities[k]);
            }
            EffectDef::CreateXXTokenEffect { token, amount } => {
                let n = v!(amount).max(0);
                let def = Arc::new(CardDef { power: n, toughness: n, ..token.card() });
                token_image(game, token, source.0);
                for _ in 0..token_copies(game, controller) {
                    let id = game.create_obj(def.clone(), controller, Zone::Library);
                    let o = game.obj_mut(id);
                    o.token = true;
                    o.trigger_order = (1 << 32) + id.0 as u64;
                    enter_battlefield(game, d, id, controller, false);
                }
            }
            EffectDef::KraulHarpoonerEffect => {
                let (me, zcc) = source;
                if game.obj(me).zone != Zone::Battlefield || game.obj(me).zcc != zcc {
                    continue;
                }
                let x = game
                    .player(controller)
                    .graveyard
                    .iter()
                    .filter(|c| game.obj(**c).def.types.contains(&CardType::Creature))
                    .count() as i32;
                boost(game, me, x, 0, Duration::EndOfTurn);
                let Some(Target::Obj(creature)) = target else { continue };
                if game.obj(creature).zone != Zone::Battlefield {
                    continue;
                }
                let message = format!("Have {} fight {}?", game.obj(me).name(), game.obj(creature).name());
                let yes = loop {
                    match ask(game, d, controller, &Query::YesNo { message: message.clone() }) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if yes {
                    // `creature.fight(source)`: the creature is dealt damage first, then deals only lethal damage
                    let pm = game.power_toughness(me).0;
                    deal_damage(game, me, Target::Obj(creature), pm, false);
                    let pc = game.power_toughness(creature).0.min(fight_lethal(game, me, creature));
                    deal_damage(game, creature, Target::Obj(me), pc, false);
                }
            }
            EffectDef::EnlightenedConfidantEffect => {
                let gained = game.player(controller).life_gained;
                let cards: Vec<ObjId> = game.player(controller).library.iter().take(1).copied().collect();
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
                    for c in yard {
                        if game.obj(c).zone == Zone::Graveyard && game.obj(c).def.mana_value() as i32 <= gained {
                            game.move_to(c, Zone::Hand);
                        }
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
                pick_x,
            } => {
                let pick = if *pick_x { &item.x.max(0) } else { pick };
                let n = match look_count {
                    Some(c) if c == "mana_spent" => game.obj(source.0).mana_spent as i32,
                    Some(c) => count(game, c, controller),
                    None => *look,
                };
                let cards: Vec<ObjId> =
                    game.player(controller).library.iter().take(n.max(0) as usize).copied().collect();
                // `LookLibraryControllerEffect`: `lookAtCards`, titled by the source
                if !cards.is_empty() {
                    let zcc = game.obj(source.0).zcc;
                    let look = crate::state::LookedAt {
                        viewer: controller,
                        source: source.0,
                        zcc,
                        bare: false,
                        cards: cards.clone(),
                    };
                    game.looked_at.push(look);
                }
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
                // picked cards going to a hidden zone are revealed
                if !chosen.is_empty()
                    && matches!(
                        picked,
                        crate::card::PutCards::Hand
                            | crate::card::PutCards::TopAny
                            | crate::card::PutCards::BottomAny
                            | crate::card::PutCards::BottomRandom
                    )
                {
                    let zcc = game.obj(source.0).zcc;
                    let cards = chosen.iter().map(|c| (*c, Zone::Library)).collect();
                    game.revealed.push(crate::state::Revealed { source: source.0, zcc, paren: false, cards });
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
            EffectDef::SacrificeEffect { filter, amount }
            | EffectDef::SacrificeOpponentsEffect { filter, amount }
            | EffectDef::SacrificeAllEffect { filter, amount } => {
                let amount = v!(amount);
                let players: Vec<Seat> = if matches!(e, EffectDef::SacrificeEffect { .. }) {
                    match target {
                        Some(Target::Player(p)) => vec![p],
                        _ => vec![],
                    }
                } else if matches!(e, EffectDef::SacrificeAllEffect { .. }) {
                    (0..game.seats()).map(|i| (controller + i) % game.seats()).collect()
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
                // the emblem's set, for its view
                if let Some((_, _, set, pick)) = game.token_made.take() {
                    game.obj_mut(id).token_image = Some((String::new(), set, pick));
                }
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
            EffectDef::BecomesCreatureTargetEffect { token, duration } => {
                for o in &objs {
                    if game.obj(*o).zone != Zone::Battlefield {
                        continue;
                    }
                    let old = game.obj(*o).def.clone();
                    let def = becomes_creature(&old, token, false);
                    let ob = game.obj_mut(*o);
                    if *duration == Duration::EndOfTurn && ob.until_end_of_turn_def.is_none() {
                        ob.until_end_of_turn_def = Some(old);
                    }
                    ob.def = Arc::new(def);
                }
            }
            EffectDef::GreatHallOfTheBiblioplexEffect { token } => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield
                    && game.obj(src).zcc == zcc
                    && !game.has_type(src, CardType::Creature)
                {
                    let old = game.obj(src).def.clone();
                    let o = game.obj_mut(src);
                    // its card again once it leaves (Unsummon)
                    if o.printed_def.is_none() {
                        o.printed_def = Some(old.clone());
                    }
                    o.def = Arc::new(becomes_creature(&old, token, true));
                }
            }
            EffectDef::BecomesCreatureSourceEffect { token, retain, duration } => {
                // the source as it is now, even once it has left and come back (`MageObjectReference(sourceId, game)`)
                let (src, _) = source;
                if game.obj(src).zone == Zone::Battlefield {
                    let old = game.obj(src).def.clone();
                    let def = becomes_creature(&old, token, *retain);
                    let o = game.obj_mut(src);
                    if *duration == Duration::EndOfTurn {
                        if o.until_end_of_turn_def.is_none() {
                            o.until_end_of_turn_def = Some(old);
                        }
                    } else if o.printed_def.is_none() {
                        // for as long as it stays: the card it was comes back when it leaves (Soulstone Sanctuary in
                        // a graveyard isn't a creature card)
                        o.printed_def = Some(o.until_end_of_turn_def.clone().unwrap_or(old));
                    }
                    o.def = Arc::new(def);
                }
            }
            EffectDef::EquipEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    if let Some(&to) = objs.first() {
                        // an Equipment that's a creature (Ensoul Artifact) doesn't attach (`AttachEffect`, 301.5c)
                        if game.obj(to).zone == Zone::Battlefield
                            && game.has_type(to, CardType::Creature)
                            && to != src
                            && !game.has_type(src, CardType::Creature)
                        {
                            let z = game.obj(to).zcc;
                            game.attach_to(src, to, z);
                        }
                    }
                }
            }
            EffectDef::SiegeDefeatedEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone != Zone::Battlefield || game.obj(src).zcc != zcc {
                    continue;
                }
                game.move_to(src, Zone::Exile);
                if game.obj(src).zone != Zone::Exile {
                    continue;
                }
                let Some(back) = game.obj(src).def.transform_back.clone() else { continue };
                let o = game.obj_mut(src);
                o.front = Some(std::mem::replace(&mut o.def, back));
                if !cast_free(game, d, controller, src) {
                    let o = game.obj_mut(src);
                    if let Some(front) = o.front.take() {
                        o.def = front;
                    }
                }
                if d.stopped() {
                    return;
                }
            }
            EffectDef::LairOfTheHydraEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    let token = crate::card::TokenDef {
                        name: String::new(),
                        class: String::new(),
                        types: vec![CardType::Creature],
                        colors: vec![Color::G],
                        subtypes: vec!["Hydra".into()],
                        power: item.x,
                        toughness: item.x,
                        abilities: Vec::new(),
                        all_creature_types: false,
                        loyalty: None,
                        images: Default::default(),
                    };
                    let old = game.obj(src).def.clone();
                    let def = becomes_creature(&old, &token, true);
                    let o = game.obj_mut(src);
                    if o.until_end_of_turn_def.is_none() {
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
                    note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
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
            // `GainProtectionFromColorAllEffect.init`: the color (CHOOSE_CHOICE), then the permanents it affects
            EffectDef::GainProtectionFromColorAllEffect { filter } => {
                let names = ["White", "Blue", "Black", "Red", "Green"];
                let choices: Vec<String> = names.iter().map(|n| n.to_string()).collect();
                let q = Query::Choice { message: "Choose color".into(), choices };
                let color = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Text(t) => {
                            if let Some(i) = names.iter().position(|n| *n == t) {
                                break Color::ALL[i];
                            }
                        }
                        Answer::Stop => return,
                        _ => {}
                    }
                    if d.stopped() {
                        return;
                    }
                };
                let ability = AbilityDef::ProtectionFromColorAbility { c: color };
                for id in Arc::clone(&game.battlefield).iter().copied() {
                    if game.filter_matches(filter, controller, Some(source.0), id) {
                        grant(game, id, &ability);
                    }
                }
            }
            EffectDef::ElvenPassageEffect => {
                let basic = crate::card::Filter(vec![
                    crate::card::Pred::Type { t: CardType::Land },
                    crate::card::Pred::Supertype { s: "BASIC".into() },
                ]);
                let Some(found) = search_library(game, d, controller, &basic, 1, 1, source.0) else { return };
                // Java shuffles before the land leaves the library
                let mut library = game.player(controller).library.clone();
                game.rng.shuffle(&mut library);
                game.player_mut(controller).library = library;
                let Some(land) = found.first().copied() else { continue };
                if !put_cards(game, d, controller, &[land], crate::card::PutCards::BattlefieldTapped) {
                    return;
                }
                let (perms, cards) = beholdable(game, controller, "Elf");
                if perms.is_empty() && cards.is_empty() {
                    continue;
                }
                let q = Query::YesNo { message: "Behold an Elf?".into() };
                let yes = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                let cost = CostDef::BeholdCost { subtype: "Elf".into() };
                if yes && pay_cost(game, d, controller, source.0, &cost) {
                    if game.obj(land).zone == Zone::Battlefield {
                        game.obj_mut(land).tapped = false;
                    }
                } else if d.stopped() {
                    return;
                }
            }
            EffectDef::LilianaDreadhordeGeneralEffect => {
                let mut keep: Vec<ObjId> = Vec::new();
                for i in 1..game.seats() {
                    let p = (controller + i) % game.seats();
                    // `CardType.values()`, the permanent types in order
                    for t in [
                        CardType::Artifact,
                        CardType::Battle,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Land,
                        CardType::Planeswalker,
                    ] {
                        let possible: Vec<ObjId> = game
                            .battlefield
                            .iter()
                            .copied()
                            .filter(|b| game.obj(*b).controller == p && game.has_type(*b, t))
                            .collect();
                        if possible.is_empty() {
                            continue;
                        }
                        match choose_cards(game, d, p, &possible, &possible, 1, 1, "keep") {
                            Some(c) => keep.extend(c),
                            None => return,
                        }
                    }
                }
                let rest: Vec<ObjId> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|b| game.obj(*b).controller != controller && !keep.contains(b))
                    .collect();
                for o in rest {
                    if game.obj(o).zone == Zone::Battlefield {
                        sacrifice(game, o);
                    }
                }
            }
            EffectDef::GainFlashbackTargetEffect => {
                let turn = game.turn;
                for o in &objs {
                    if game.obj(*o).zone == Zone::Graveyard {
                        game.obj_mut(*o).flashback_turn = Some(turn);
                    }
                }
            }
            EffectDef::TwinflameTyrantEffect | EffectDef::CantBeBlockedByCreaturesWithLessPowerEffect => {}
            EffectDef::MaraudingRaptorEffect => {
                let Some(&o) = objs.first() else { continue };
                if game.obj(o).zone != Zone::Battlefield {
                    continue;
                }
                if deal_damage(game, source.0, Target::Obj(o), 2, false) > 0 && game.has_subtype(o, "Dinosaur") {
                    let (src, zcc) = source;
                    if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                        boost(game, src, 2, 0, Duration::EndOfTurn);
                    }
                }
            }
            EffectDef::SolveEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    game.obj_mut(src).solved = true;
                }
            }
            EffectDef::CaseOfTheUneatenFeastEffect => {
                let turn = game.turn;
                let cards: Vec<ObjId> = game
                    .player(controller)
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|c| game.obj(*c).is(CardType::Creature))
                    .collect();
                for c in cards {
                    game.obj_mut(c).may_play = Some((controller, turn));
                    game.obj_mut(c).cast_only = true;
                }
            }
            EffectDef::MayCastTargetCardEffect { any_mana } => {
                let Some(&card) = objs.first() else { continue };
                let q = Query::YesNo { message: format!("Cast {}?", game.obj(card).def.name) };
                let yes = loop {
                    match ask(game, d, controller, &q) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if !yes {
                    continue;
                }
                let was = game.obj(card).any_mana;
                if *any_mana {
                    game.obj_mut(card).any_mana = true;
                }
                if !cast_allowed(game, d, controller, card) {
                    game.obj_mut(card).any_mana = was;
                }
                if d.stopped() {
                    return;
                }
            }
            EffectDef::DamageMultiEffect => {
                let chosen = item.targets.first().cloned().unwrap_or_default();
                let ok = legal.first().cloned().unwrap_or_default();
                for (t, n) in chosen.iter().zip(item.target_amounts.iter()) {
                    if ok.contains(t) {
                        deal_damage(game, source.0, *t, *n, false);
                    }
                }
            }
            EffectDef::DistributeCountersEffect | EffectDef::DistributeM1M1CountersEffect => {
                let counter = if matches!(e, EffectDef::DistributeM1M1CountersEffect) { "-1/-1" } else { "+1/+1" };
                let chosen = item.targets.first().cloned().unwrap_or_default();
                let ok = legal.first().cloned().unwrap_or_default();
                for (t, n) in chosen.iter().zip(item.target_amounts.iter()) {
                    if let Target::Obj(o) = t {
                        if ok.contains(t) && game.obj(*o).zone == Zone::Battlefield {
                            game.obj_mut(*o).add_counters(counter, *n);
                        }
                    }
                }
            }
            EffectDef::WitchsOvenEffect { token } => {
                let n = if item.sacrificed_toughness.unwrap_or(0) > 3 { 2 } else { 1 };
                let e = EffectDef::CreateTokenEffect {
                    token: token.clone(),
                    amount: Amount::Fixed(n),
                    tapped_and_attacking: false,
                    tapped: false,
                    attacking: false,
                    sacrifice_at_end_of_combat: false,
                    sacrifice_at_next_end_step: false,
                    haste_until_end_of_turn: false,
                    also: None,
                    attach_source: None,
                };
                apply_effects(game, d, item, std::slice::from_ref(&e), legal, source);
            }
            EffectDef::RighteousValkyrieEffect => {
                if let Some(e) = item.entering {
                    // a creature already gone (the legend rule) counts as it last was
                    let o = game.obj(e);
                    let toughness = match (o.zone, o.toughness_lki) {
                        (Zone::Battlefield, _) | (_, None) => game.power_toughness(e).1,
                        (_, Some(t)) => t,
                    };
                    gain_life(game, controller, toughness);
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
                for cards in groups {
                    let owner = game.obj(cards[0]).owner;
                    // `PlayerImpl.moveCards` puts them on top one at a time, unasked, before the shuffle
                    for c in cards {
                        to_library(game, c, true);
                    }
                    let mut library = game.player(owner).library.clone();
                    game.rng.shuffle(&mut library);
                    game.player_mut(owner).library = library;
                }
            }
            EffectDef::SuspendRemoveTimeCounterEffect => {
                let (card, zcc) = source;
                let o = game.obj(card);
                if o.zone == Zone::Exile && o.zcc == zcc && o.suspended && o.counter("time") > 0 {
                    let o = game.obj_mut(card);
                    o.add_counters("time", -1);
                    o.counters.retain(|(n, k)| n != "time" || *k > 0);
                    if game.obj(card).counter("time") == 0 {
                        let fired = game.triggers([card], |_, _, _, t| matches!(t, Trigger::SuspendLastCounter));
                        game.pending.extend(fired);
                    }
                }
            }
            EffectDef::ReboundCastSpellFromExileEffect => {
                let (card, zcc) = source;
                if game.obj(card).zone != Zone::Exile || game.obj(card).zcc != zcc {
                    continue;
                }
                let owner = game.obj(card).owner;
                // `CardUtil.castSpellWithAttributesForFree`: only a card that can be cast now is offered
                let free = CardDef { cost: ManaCost::default(), ..(*game.obj(card).def).clone() };
                if !has_targets(game, owner, card, &free) {
                    continue;
                }
                let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
                let yes = loop {
                    match ask(game, d, owner, &Query::YesNo { message: message.clone() }) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if yes {
                    cast_free(game, d, owner, card);
                    if d.stopped() {
                        return;
                    }
                }
            }
            EffectDef::SuspendPlayCardEffect => {
                let (card, zcc) = source;
                if game.obj(card).zone != Zone::Exile || game.obj(card).zcc != zcc {
                    continue;
                }
                // with no time counter left it isn't suspended any more, cast or not (702.62b)
                game.obj_mut(card).suspended = false;
                let owner = game.obj(card).owner;
                // `CardUtil.castSpellWithAttributes`: only a card that can be cast now is offered
                let free = CardDef { cost: ManaCost::default(), ..(*game.obj(card).def).clone() };
                if !has_targets(game, owner, card, &free) {
                    continue;
                }
                let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
                let yes = loop {
                    match ask(game, d, owner, &Query::YesNo { message: message.clone() }) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if yes {
                    cast_free(game, d, owner, card);
                    if d.stopped() {
                        return;
                    }
                }
            }
            EffectDef::AetherVialEffect => {
                let n = game.obj(source.0).counter("charge") as u32;
                let fits: Vec<ObjId> = game
                    .player(controller)
                    .hand
                    .iter()
                    .copied()
                    .filter(|c| game.obj(*c).is(CardType::Creature) && game.obj(*c).def.mana_value() == n)
                    .collect();
                if fits.is_empty() {
                    continue;
                }
                let message =
                    format!("Put a creature card with mana value equal to {n} from your hand onto the battlefield?");
                let yes = loop {
                    match ask(game, d, controller, &Query::YesNo { message: message.clone() }) {
                        Answer::Bool(b) => break b,
                        Answer::Stop => return,
                        _ => {}
                    }
                };
                if !yes {
                    continue;
                }
                let hand = game.player(controller).hand.clone();
                let Some(chosen) = choose_cards(game, d, controller, &hand, &fits, 1, 1, "put onto the battlefield")
                else {
                    return;
                };
                for c in chosen {
                    enter_battlefield(game, d, c, controller, false);
                }
            }
            EffectDef::TideShaperEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone != Zone::Battlefield || game.obj(src).zcc != zcc {
                    continue;
                }
                for o in &objs {
                    if game.obj(*o).zone == Zone::Battlefield && game.obj(*o).is(CardType::Land) {
                        let lz = game.obj(*o).zcc;
                        game.land_becomes.push((*o, lz, src, zcc, "Island".into()));
                    }
                }
                refresh_land_types(game);
            }
            EffectDef::RangerCaptainOfEosEffect => {
                let turn = game.turn;
                for p in 0..game.seats() {
                    if p != controller {
                        game.player_mut(p).no_noncreature_turn = Some(turn);
                    }
                }
            }
            EffectDef::SearingBlazeEffect => {
                let n = if game.player(controller).lands_entered > 0 { 3 } else { 1 };
                for i in 0..2 {
                    for t in legal.get(i).cloned().unwrap_or_default() {
                        deal_damage(game, source.0, t, n, false);
                    }
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
                    if *duration == Duration::EndOfTurn {
                        if o.until_end_of_turn_def.is_none() {
                            o.until_end_of_turn_def = Some(old);
                        }
                    } else if o.printed_def.is_none() {
                        // for as long as it stays: the card it was comes back when it leaves (Soulstone Sanctuary in
                        // a graveyard isn't a creature card)
                        o.printed_def = Some(o.until_end_of_turn_def.clone().unwrap_or(old));
                    }
                    o.def = Arc::new(def);
                }
            }
            EffectDef::KothTheGeomancerEffect => {
                let opponent = game.opponent(controller);
                deal_damage(game, source.0, Target::Player(opponent), 1, false);
                if objs.first().is_some_and(|l| game.obj(*l).def.has_subtype("Mountain")) {
                    game.player_mut(controller).mana_pool.colored[Color::R.index()] += 1;
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
            EffectDef::RemoveUpToAmountCountersEffect { amount } => {
                let Some(Target::Obj(t)) = target else { continue };
                if game.obj(t).zone != Zone::Battlefield || *amount < 1 {
                    continue;
                }
                let names = java_hash_order(
                    game.obj(t).counters.iter().filter(|(_, n)| *n > 0).map(|(c, _)| c.clone()).collect(),
                );
                let n = names.len();
                if n == 0 {
                    continue;
                }
                let Some(values) = multi_amount(
                    game,
                    d,
                    controller,
                    vec![0; n],
                    vec![(0, *amount); n],
                    (0, *amount),
                    (None, Vec::new()),
                ) else {
                    continue;
                };
                for (name, v) in names.iter().zip(values) {
                    let o = game.obj_mut(t);
                    if let Some(c) = o.counters.iter_mut().find(|(c, _)| c == name) {
                        c.1 -= v.min(c.1).max(0);
                    }
                }
            }
            EffectDef::EnduringGlimmerReturnEffect => {
                let card = source.0;
                if matches!(game.obj(card).zone, Zone::Battlefield | Zone::Stack) {
                    continue;
                }
                let old = game.obj(card).def.clone();
                let mut def = (*old).clone();
                def.types = vec![CardType::Enchantment];
                def.subtypes.clear();
                let o = game.obj_mut(card);
                o.def = Arc::new(def);
                o.printed_def = Some(old.clone());
                game.enters_as = Some((card, old));
                enter_battlefield(game, d, card, controller, false);
                game.enters_as = None;
            }
            EffectDef::AddExtraTurnControllerEffect => game.extra_turns.push(controller),
            EffectDef::ExileSpellEffect => {
                if game.obj(source.0).zone == Zone::Stack {
                    game.obj_mut(source.0).flashback = true;
                }
            }
            EffectDef::NinjutsuEffect => {
                let card = source.0;
                let defender = game.obj_mut(card).ninjutsu_defender.take();
                // attacking what the returned creature attacked, a planeswalker too
                let pw = game.obj_mut(card).ninjutsu_pw.take();
                if game.obj(card).zone != Zone::Hand {
                    continue;
                }
                enter_battlefield(game, d, card, controller, false);
                let o = game.obj_mut(card);
                o.tapped = true;
                o.attacking = defender;
                o.attacking_pw = pw;
                if defender.is_some() {
                    join_combat(game, card);
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
            EffectDef::CantBeBlockedByCreaturesSourceEffect { filter, .. } => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    grant(game, src, &AbilityDef::CantBeBlockedByAbility { filter: filter.clone() });
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
            EffectDef::ExileTargetForSourceEffect => {
                let (src, zcc) = source;
                for o in objs.iter().copied() {
                    if matches!(game.obj(o).zone, Zone::Battlefield | Zone::Graveyard) {
                        game.move_to(o, Zone::Exile);
                        game.exiled_for_source.push((src, zcc, o));
                    }
                }
            }
            EffectDef::CreateXXTokenExiledEffect { token } => {
                let (src, zcc) = source;
                let exiled: Vec<ObjId> = game
                    .exiled_for_source
                    .iter()
                    // a dies trigger has its graveyard card's zone change counter: the permanent's was one less
                    .filter(|(s, z, c)| *s == src && (*z == zcc || *z + 1 == zcc) && game.obj(*c).zone == Zone::Exile)
                    .map(|(_, _, c)| *c)
                    .collect();
                let total: i32 = exiled.iter().map(|c| game.obj(*c).def.mana_value() as i32).sum();
                let mut owners: Vec<Seat> = exiled.iter().map(|c| game.obj(*c).owner).collect();
                owners.dedup();
                let owners: Vec<Seat> =
                    owners.into_iter().flat_map(|p| std::iter::repeat_n(p, token_copies(game, p))).collect();
                for owner in owners {
                    let def = Arc::new(CardDef { power: total, toughness: total, ..token.card() });
                    token_image(game, token, src);
                    let id = game.create_obj(def, owner, Zone::Library);
                    let o = game.obj_mut(id);
                    o.token = true;
                    o.trigger_order = (1 << 32) + id.0 as u64;
                    enter_battlefield(game, d, id, owner, false);
                }
            }
            EffectDef::EliteSpellbinderEffect => {
                let Some(Target::Player(p)) = target else { continue };
                let hand = game.player(p).hand.clone();
                if hand.is_empty() {
                    continue;
                }
                let possible: Vec<ObjId> = hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
                let rule_text = item_rule(game, item);
                let Some(chosen) = choose_cards(game, d, controller, &hand, &possible, 0, 1, &rule_text) else {
                    return;
                };
                for c in chosen {
                    game.move_to(c, Zone::Exile);
                    let owner = game.obj(c).owner;
                    let o = game.obj_mut(c);
                    o.may_play = Some((owner, u32::MAX));
                    o.play_tax = 2;
                }
            }
            EffectDef::CreateNextEndStepDelayedEffect { effects, rule } => {
                let delayed = Delayed {
                    rule: rule.clone(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: effects.clone(),
                    source_zcc: source.1,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                game.at_next_end_step
                    .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
            }
            EffectDef::CreateNextUpkeepDelayedEffect { effects, rule } => {
                let delayed = Delayed {
                    rule: rule.clone(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: effects.clone(),
                    source_zcc: source.1,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                game.at_next_upkeep
                    .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(source.0, controller) });
            }
            // Mishra's Bauble: its controller looks at the top card of the target player's library
            EffectDef::LookLibraryTopCardTargetPlayerEffect => {
                let Some(Target::Player(p)) = target else { continue };
                let cards: Vec<ObjId> = game.player(p).library.iter().take(1).copied().collect();
                if !cards.is_empty() {
                    let zcc = game.obj(source.0).zcc;
                    let look = crate::state::LookedAt { viewer: controller, source: source.0, zcc, bare: true, cards };
                    game.looked_at.push(look);
                }
            }
            EffectDef::ExpressiveIterationEffect => {
                let mut cards: Vec<ObjId> = game.player(controller).library.iter().take(3).copied().collect();
                if cards.is_empty() {
                    return;
                }
                let min = usize::from(cards.len() == 3);
                let rule_text = item_rule(game, item);
                let Some(hand) = choose_cards(game, d, controller, &cards.clone(), &cards.clone(), min, 1, &rule_text)
                else {
                    return;
                };
                for c in hand {
                    game.move_to(c, Zone::Hand);
                    cards.retain(|x| *x != c);
                }
                if cards.is_empty() {
                    return;
                }
                let min = usize::from(cards.len() == 2);
                let rule_text = item_rule(game, item);
                let Some(bottom) =
                    choose_cards(game, d, controller, &cards.clone(), &cards.clone(), min, 1, &rule_text)
                else {
                    return;
                };
                for c in bottom {
                    to_library(game, c, false);
                    cards.retain(|x| *x != c);
                }
                if cards.is_empty() {
                    return;
                }
                let rule_text = item_rule(game, item);
                let Some(exiled) = choose_cards(game, d, controller, &cards.clone(), &cards.clone(), 1, 1, &rule_text)
                else {
                    return;
                };
                for c in exiled {
                    game.move_to(c, Zone::Exile);
                    if game.obj(c).zone == Zone::Exile {
                        game.obj_mut(c).exiled_with = Some(source);
                        game.obj_mut(c).may_play = Some((controller, game.turn));
                    }
                }
            }
            EffectDef::UntapLandsEffect { amount } => {
                // `UntapLandsEffect`: any tapped land; with as few of yours tapped as it untaps, it first offers them
                let tapped: Vec<ObjId> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|b| game.obj(*b).is(CardType::Land) && game.obj(*b).tapped)
                    .collect();
                let yours: Vec<ObjId> =
                    tapped.iter().copied().filter(|b| game.obj(*b).controller == controller).collect();
                let mut chosen: Vec<ObjId> = Vec::new();
                if yours.len() <= *amount {
                    let q = Query::YesNo { message: "Include your tapped lands to untap?".into() };
                    let yes = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Bool(b) => break b,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    if yes {
                        chosen = yours.clone();
                    }
                }
                // `TargetImpl.choose` starts with `chosen` false, so even a full preselection is asked
                // (`isChoiceSelected`); a pick past the maximum adds nothing. It asks again while the player keeps
                // adding, until the choice is complete
                let mut selected = false;
                loop {
                    let prev = chosen.len();
                    let complete =
                        |chosen: &Vec<ObjId>| chosen.len() >= *amount || tapped.iter().all(|t| chosen.contains(t));
                    let mut picked = false;
                    if !(selected && complete(&chosen)) {
                        loop {
                            let q = Query::Target { players: Vec::new(), objs: tapped.clone(), required: false };
                            match ask(game, d, controller, &q) {
                                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                                Answer::Obj(o) if tapped.contains(&o) => {
                                    if chosen.len() < *amount {
                                        chosen.push(o);
                                        if complete(&chosen) {
                                            picked = true;
                                            break;
                                        }
                                    }
                                }
                                Answer::Bool(_) => {
                                    picked = !chosen.is_empty();
                                    break;
                                }
                                Answer::Stop => return,
                                _ => {}
                            }
                        }
                    }
                    if !picked {
                        break;
                    }
                    selected = true;
                    if complete(&chosen) || prev == chosen.len() {
                        break;
                    }
                }
                for o in chosen {
                    game.obj_mut(o).tapped = false;
                }
            }
            EffectDef::PutIntoLibraryNFromTopTargetEffect { n } => {
                for o in objs.iter().copied() {
                    if game.obj(o).zone != Zone::Battlefield {
                        continue;
                    }
                    let owner = game.obj(o).owner;
                    let size = game.player(owner).library.len();
                    game.move_to(o, Zone::Library);
                    // a token ceases to exist (it's in no library's list)
                    if size + 1 >= *n && game.obj(o).zone == Zone::Library && !game.obj(o).token {
                        let lib = &mut game.player_mut(owner).library;
                        lib.retain(|x| *x != o);
                        lib.insert((*n - 1).min(lib.len()), o);
                    }
                }
            }
            EffectDef::ScreamingNemesisEffect => {
                let Some(t) = target else { continue };
                let dealt = deal_damage(game, source.0, t, item.x, false);
                if let Target::Player(p) = t {
                    if dealt > 0 {
                        game.player_mut(p).cant_gain_life = true;
                    }
                }
            }
            EffectDef::BecomesSubtypesSourceEffect { subtypes } => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    game.obj_mut(src).creature_types = Some(subtypes.clone());
                }
            }
            EffectDef::SetBasePowerToughnessUntilLeavesEffect { power, toughness } => {
                let (src, zcc) = source;
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    game.obj_mut(src).base_pt = Some((*power, *toughness));
                }
            }
            EffectDef::EvolvedSleeperClericEffect => {
                let (src, zcc) = source;
                if game.obj(src).zone != Zone::Battlefield
                    || game.obj(src).zcc != zcc
                    || !game.has_subtype(src, "Cleric")
                {
                    continue;
                }
                game.obj_mut(src).add_counters("deathtouch", 1);
                let o = game.obj_mut(src);
                o.creature_types = Some(["Phyrexian", "Human", "Cleric"].map(String::from).to_vec());
                o.base_pt = Some((3, 3));
            }
            EffectDef::EvolvedSleeperPhyrexianEffect => {
                let (src, zcc) = source;
                // (gone: Java looks at what it last was; its added subtypes aren't kept here)
                if game.obj(src).zone != Zone::Battlefield
                    || game.obj(src).zcc != zcc
                    || !game.has_subtype(src, "Phyrexian")
                {
                    continue;
                }
                game.obj_mut(src).add_counters("+1/+1", 1);
                draw(game, controller, 1);
                lose_life(game, controller, 1);
            }
            EffectDef::GraveyardTrespasserEffect => {
                let cards: Vec<ObjId> = objs.iter().copied().filter(|o| game.obj(*o).zone == Zone::Graveyard).collect();
                let mut amount = 0;
                for o in cards {
                    let creature = game.obj(o).is(CardType::Creature);
                    game.move_to(o, Zone::Exile);
                    if creature && game.obj(o).zone == Zone::Exile {
                        amount += 1;
                    }
                }
                if amount > 0 {
                    gain_life(game, controller, amount);
                    for s in 0..game.seats() {
                        if s != controller {
                            lose_life(game, s, amount);
                        }
                    }
                }
            }
            EffectDef::RavenEagleEffect { then } => {
                let Some(Target::Obj(o)) = target else { continue };
                if game.obj(o).zone != Zone::Graveyard {
                    continue;
                }
                let creature = game.obj(o).is(CardType::Creature);
                game.move_to(o, Zone::Exile);
                if creature {
                    apply_effects(game, d, item, std::slice::from_ref(&**then), &[], source);
                }
            }
            EffectDef::KitesailFreebooterExileEffect { filter } => {
                let (src, zcc) = source;
                let Some(Target::Player(p)) = target else { continue };
                let hand = game.player(p).hand.clone();
                let possible: Vec<ObjId> =
                    hand.iter().copied().filter(|c| game.filter_matches(filter, controller, Some(src), *c)).collect();
                if possible.is_empty() {
                    continue;
                }
                let rule_text = item_rule(game, item);
                let Some(chosen) = choose_cards(game, d, controller, &hand, &possible, 1, 1, &rule_text) else {
                    return;
                };
                // the card isn't exiled if this left the battlefield first
                if game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    for c in chosen {
                        game.move_to(c, Zone::Exile);
                        game.exiled_until_leaves.push((src, zcc, c));
                        game.return_to_hand.push(c);
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
                    if !hand.is_empty() {
                        let zcc = game.obj(source.0).zcc;
                        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
                        game.revealed.push(crate::state::Revealed { source: source.0, zcc, paren: true, cards });
                    }
                    if !possible.is_empty() {
                        // the spell's text: your own card to discard isn't chosen by itself
                        let rule = item_rule(game, item);
                        match choose_cards(game, d, controller, &hand, &possible, 1, 1, &rule) {
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
            EffectDef::CacheGrabEffect { token } => {
                let milled: Vec<ObjId> = game.player(controller).library.iter().take(4).copied().collect();
                for c in &milled {
                    game.move_to(*c, Zone::Graveyard);
                }
                let filter = crate::card::Filter(vec![Pred::PermanentCard]);
                let possible: Vec<ObjId> = milled
                    .iter()
                    .copied()
                    .filter(|c| game.filter_matches(&filter, controller, Some(source.0), *c))
                    .collect();
                let mut squirrel = false;
                if !possible.is_empty() {
                    let rule_text = item_rule(game, item);
                    match choose_cards(game, d, controller, &milled, &possible, 0, 1, &rule_text) {
                        Some(chosen) => {
                            for c in chosen {
                                game.move_to(c, Zone::Hand);
                                squirrel |= game.has_subtype(c, "Squirrel");
                            }
                        }
                        None => return,
                    }
                }
                squirrel |= game
                    .battlefield
                    .iter()
                    .any(|p| game.obj(*p).controller == controller && game.has_subtype(*p, "Squirrel"));
                if squirrel {
                    let e = EffectDef::CreateTokenEffect {
                        token: token.clone(),
                        amount: Amount::Fixed(1),
                        tapped_and_attacking: false,
                        tapped: false,
                        attacking: false,
                        sacrifice_at_end_of_combat: false,
                        sacrifice_at_next_end_step: false,
                        haste_until_end_of_turn: false,
                        also: None,
                        attach_source: None,
                    };
                    apply_effects(game, d, item, std::slice::from_ref(&e), legal, source);
                }
            }
            EffectDef::ScavengersTalentEffect => {
                let graveyard = game.player(controller).graveyard.clone();
                let possible: Vec<ObjId> =
                    graveyard.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
                if !possible.is_empty() {
                    let rule_text = item_rule(game, item);
                    match choose_cards(game, d, controller, &graveyard, &possible, 1, 1, &rule_text) {
                        Some(chosen) => {
                            for c in chosen {
                                if game.obj(c).zone == Zone::Graveyard {
                                    enter_battlefield(game, d, c, controller, false);
                                    game.obj_mut(c).add_counters("finality", 1);
                                }
                            }
                        }
                        None => return,
                    }
                }
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
                    let rule_text = item_rule(game, item);
                    match choose_cards(game, d, controller, &graveyard, &possible, min, 1, &rule_text) {
                        Some(chosen) => {
                            for c in chosen {
                                game.move_to(c, Zone::Hand);
                            }
                        }
                        None => return,
                    }
                }
            }
            EffectDef::PutOnLibraryTargetEffect { top } => {
                for o in &objs {
                    // a card from a graveyard, or a permanent (Natural Obsolescence's artifact; a token ceases to exist)
                    if matches!(game.obj(*o).zone, Zone::Graveyard | Zone::Battlefield) {
                        to_library(game, *o, *top);
                    }
                }
            }
            EffectDef::RevealLibraryPickControllerEffect { amount, pick, filter, optional, to } => {
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
                        let rule_text = item_rule(game, item);
                        // `LookLibraryAndPickControllerEffect`: picking more than one is "up to" (none at least)
                        let min = if *pick > 1 { 0 } else { number };
                        match choose_cards(game, d, controller, &cards, &possible, min, number, &rule_text) {
                            Some(chosen) => picked = chosen,
                            None => return,
                        }
                    }
                }
                if *to == crate::card::PutCards::Hand {
                    for c in &picked {
                        game.move_to(*c, Zone::Hand);
                    }
                } else if !picked.is_empty() && !put_cards(game, d, controller, &picked, *to) {
                    return;
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
            EffectDef::CreateTokenTargetEffect { token, amount } => {
                let def = Arc::new(token.card());
                let n = match amount {
                    Some(a) => v!(a).max(0) as usize,
                    None => 1,
                };
                if n > 0 {
                    token_image(game, token, source.0);
                }
                let each: Vec<Seat> = players
                    .clone()
                    .into_iter()
                    .flat_map(|p| std::iter::repeat_n(p, n * token_copies(game, p)))
                    .collect();
                for p in each {
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
                let each: Vec<Seat> = to
                    .into_iter()
                    .flat_map(|p| std::iter::repeat_n(p, *amount as usize * token_copies(game, p)))
                    .collect();
                for p in each {
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
            EffectDef::ProliferateEffect => {
                if !proliferate(game, d, controller) {
                    return;
                }
            }
            EffectDef::PopulateEffect => {
                // `PopulateEffect`: `choose` a creature token you control (not targeted), then copy it
                let tokens: Vec<ObjId> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|o| {
                        game.obj(*o).token
                            && game.obj(*o).controller == controller
                            && game.has_type(*o, CardType::Creature)
                    })
                    .collect();
                if !tokens.is_empty() {
                    let rule = item_rule(game, item);
                    let Some(chosen) = choose_cards(game, d, controller, &tokens, &tokens, 1, 1, &rule) else {
                        return;
                    };
                    if let Some(&o) = chosen.first() {
                        if log::kept(game) {
                            let line = format!("Token selected for populate: {}", log::object(game, o));
                            log::inform(game, line);
                        }
                        token_copy(game, d, o, controller, None);
                    }
                }
            }
            EffectDef::AmassEffect { amount, subtype, token } => {
                // `AmassEffect.doAmass`
                let n = v!(amount);
                let army = |g: &Game| -> Vec<ObjId> {
                    g.battlefield
                        .iter()
                        .copied()
                        .filter(|o| g.obj(*o).controller == controller && g.has_subtype(*o, "Army"))
                        .collect()
                };
                if army(game).is_empty() {
                    let e = EffectDef::CreateTokenEffect {
                        token: token.clone(),
                        amount: Amount::Fixed(1),
                        tapped_and_attacking: false,
                        tapped: false,
                        attacking: false,
                        sacrifice_at_end_of_combat: false,
                        sacrifice_at_next_end_step: false,
                        haste_until_end_of_turn: false,
                        also: None,
                        attach_source: None,
                    };
                    apply_effects(game, d, item, &[e], legal, source);
                }
                let armies = army(game);
                // one Army: no prompt
                let chosen = match armies.len() {
                    0 => None,
                    1 => Some(armies[0]),
                    _ => {
                        let rule = item_rule(game, item);
                        let Some(c) = choose_cards(game, d, controller, &armies, &armies, 1, 1, &rule) else {
                            return;
                        };
                        c.first().copied()
                    }
                };
                if let Some(o) = chosen {
                    if !game.has_subtype(o, subtype) {
                        let zcc = game.obj(o).zcc;
                        game.subtypes_while_there.push((o, zcc, subtype.clone()));
                    }
                    if n > 0 {
                        put_counters(game, o, "+1/+1", n, controller);
                    }
                }
            }
            EffectDef::DontUntapInControllersNextUntapStepTargetEffect => {
                for o in &objs {
                    if game.obj(*o).zone == Zone::Battlefield {
                        let zcc = game.obj(*o).zcc;
                        if !game.skip_next_untap.contains(&(*o, zcc)) {
                            game.skip_next_untap.push((*o, zcc));
                        }
                    }
                }
            }
            EffectDef::WinGameSourceControllerEffect => {
                // `PlayerImpl.won`: each opponent still in loses (`lostForced`), then, with none left, you win
                if !game.over && !game.player(controller).lost {
                    for s in 0..game.seats() {
                        if s != controller && !game.player(s).lost {
                            game.player_mut(s).lost = true;
                            if log::kept(game) {
                                let line = format!("{} has lost the game", log::player(s));
                                log::inform(game, line);
                            }
                        }
                    }
                    if log::kept(game) {
                        let line = format!("{} has won the game", log::player(controller));
                        log::inform(game, line);
                    }
                    game.player_mut(controller).won = true;
                    game.over = true;
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
            EffectDef::MillThenPutInHandEffect { amount, filter, optional, otherwise } => {
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
                // `HumanPlayer.choose(cards)` shows the milled cards even when none of them can be chosen
                let mut returned = false;
                if !milled.is_empty() {
                    let min = if *optional || possible.is_empty() { 0 } else { 1 };
                    let rule_text = item_rule(game, item);
                    match choose_cards(game, d, controller, &milled, &possible, min, 1, &rule_text) {
                        Some(chosen) => {
                            returned = !chosen.is_empty();
                            for c in chosen {
                                game.move_to(c, Zone::Hand);
                            }
                        }
                        None => return,
                    }
                }
                // `applyOtherwiseEffect`: nothing milled, or nothing put into the hand
                if let (false, Some(e)) = (returned, otherwise) {
                    apply_effects(game, d, item, std::slice::from_ref(&**e), legal, source);
                }
            }
            EffectDef::MillCardsControllerEffect { amount } => {
                let amount = &v!(amount);
                let n = (*amount).max(0) as usize;
                let top: Vec<ObjId> = game.player(controller).library.iter().take(n).copied().collect();
                note(game, EventKind::Milled, Some(controller), None, None, top.len() as i32);
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
                let paying = game.stack[at].card;
                let Some(paid) = pay_unless(game, d, payer, paying, cost) else { return };
                if !paid {
                    Arc::make_mut(&mut game.stack).remove(at);
                }
            }
            EffectDef::CounterUnlessPaysEffect { cost, generic, exile } => {
                let Some(Target::Obj(spell)) = target else { continue };
                let cost = match generic {
                    Some(a) => vec![CostDef::ManaCost { cost: format!("{{{}}}", v!(a).max(0)) }],
                    None => cost.clone(),
                };
                let cost = &cost;
                if game.obj(spell).zone != Zone::Stack {
                    continue;
                }
                let payer = game.stack.iter().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.controller);
                let Some(payer) = payer else { continue };
                let Some(paid) = pay_unless(game, d, payer, spell, cost) else { return };
                if !paid && !cant_be_countered(game, spell) {
                    note_countered(game, spell, item.card);
                    Arc::make_mut(&mut game.stack).retain(|i| i.card != spell || i.ability.is_some());
                    game.move_to(spell, if *exile { Zone::Exile } else { Zone::Graveyard });
                }
            }
            // resolving, "{this} becomes a Dragon until end of turn" (Sarkhan, Dragon Ascendant: the pool's only one
            // that resolves); as a static ability it's `Game::added_subtypes`'
            EffectDef::AddCardSubTypeSourceEffect { subtype } => {
                let (src, zcc) = source;
                if !subtype.is_empty() && game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc {
                    game.subtypes_until_eot.push((src, zcc, subtype.clone()));
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
            // a spell's (Energybending): the controller's lands gain all basic land types until end of turn (a
            // static one, Leyline of the Guildpact, is `Game::guildpact`)
            EffectDef::BecomesAllBasicsControlledEffect => {
                if !game.all_basics_until_eot.contains(&item.controller) {
                    game.all_basics_until_eot.push(item.controller);
                }
                game.update_guildpact();
            }
            EffectDef::ExileTargetIfDiesEffect => {
                if let Some(Target::Obj(o)) = target {
                    let zcc = game.obj(o).zcc;
                    game.exile_if_dies.push((o, zcc));
                } else if target.is_none() {
                    // no target (Anger of the Gods): each creature this source dealt damage to this turn
                    let hit: Vec<(ObjId, u32)> = game
                        .damaged_by_source
                        .iter()
                        .filter(|(s, _, _)| *s == source.0)
                        .map(|(_, o, z)| (*o, *z))
                        .collect();
                    game.exile_if_dies.extend(hit);
                }
            }
            // static abilities (`Game::power_toughness`, `Game::has_ability`) and as-enters effects
            // (`enter_battlefield`)
            EffectDef::BoostControlledEffect { .. }
            | EffectDef::GainAbilityAllEffect { .. }
            | EffectDef::GainAbilityControlledEffect { .. }
            | EffectDef::ConditionalContinuousEffect { .. }
            | EffectDef::SetBasePowerToughnessSourceEffect { .. }
            | EffectDef::SetBasePowerToughnessAmountSourceEffect { .. }
            | EffectDef::CantBeCounteredSourceEffect
            | EffectDef::NowhereToRunHexproofEffect
            | EffectDef::NowhereToRunWardEffect
            | EffectDef::TenaciousUnderdogEffect
            | EffectDef::BoostAllEffect { .. }
            | EffectDef::TapSourceUnlessPaysEffect { .. }
            | EffectDef::ChooseBasicLandTypeEffect
            | EffectDef::ChooseCreatureTypeEffect
            | EffectDef::CavernOfSoulsCantCounterEffect
            | EffectDef::MultiversalPassagePassageEffect
            | EffectDef::QuantumRiddlerReplacementEffect
            | EffectDef::ArtistsTalentEffect { .. }
            | EffectDef::GainAbilityControllerEffect { .. }
            | EffectDef::LeylineOfTheVoidEffect
            | EffectDef::CantAttackAloneSourceEffect
            | EffectDef::CantBlockAloneSourceEffect
            | EffectDef::DontUntapInControllersUntapStepEnchantedEffect
            | EffectDef::MustBeBlockedByAtLeastOneSourceEffect
            | EffectDef::ValgavothTerrorEaterReplacementEffect
            | EffectDef::ValgavothTerrorEaterPlayEffect
            | EffectDef::CantAttackBlockUnlessConditionSourceEffect { .. }
            | EffectDef::EntersTappedChooseColorEffect
            | EffectDef::LeylineOfTheGuildpactEffect
            | EffectDef::BecomesCreatureAttachedEffect { .. }
            | EffectDef::InfoEffect
            | EffectDef::GiadaFontOfHopeEntersBattlefieldEffect
            | EffectDef::AddBasicLandTypeAllLandsEffect { .. }
            | EffectDef::AddCardSubtypeAllEffect { .. }
            | EffectDef::RottenmouthViperCostReductionEffect
            | EffectDef::PetrifiedHamletCostEffect
            | EffectDef::PithingNeedleEffect
            | EffectDef::TapSourceUnlessBeholdEffect { .. }
            | EffectDef::TapSourceUnlessRevealsEffect { .. }
            | EffectDef::GainAbilityAttachedEffect { .. }
            | EffectDef::EtchingOfKumanoReplacementEffect
            | EffectDef::AttachEffect
            | EffectDef::BoostEnchantedEffect { .. }
            | EffectDef::CantAttackBlockAttachedEffect
            | EffectDef::CantBeBlockedByMoreThanOneAttachedEffect
            | EffectDef::MockingbirdEffect
            | EffectDef::GraveyardFromAnywhereExileReplacementEffect
            | EffectDef::SetBasePowerSourceEffect { .. }
            | EffectDef::ValleyFlamecallerEffect
            | EffectDef::TheWanderingEmperorEffect
            | EffectDef::LoseCreatureTypeSourceEffect { .. }
            | EffectDef::AttacksIfAbleAllEffect { .. }
            | EffectDef::SoulScarMageDamageReplacementEffect => {}
        }
    }
}

/// A spell's or ability's rule text (auto-targeting reads it).
fn item_rule(game: &Game, item: &StackItem) -> String {
    let def = game.item_def(item);
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
    choose_cards_with(game, d, seat, shown, possible, min, max, rule, false)
}

/// `TargetImpl.choose` (a cost's "any number of cards", not a target): `HumanPlayer.choose` rounds, each picking
/// until the player is done or nothing more can be chosen, and another round after one that added a card. None: the
/// decider stopped.
fn choose_cards_rounds(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    possible: &[ObjId],
    min: usize,
    max: usize,
) -> Option<Vec<ObjId>> {
    let mut chosen: Vec<ObjId> = Vec::new();
    let complete = |chosen: &Vec<ObjId>| {
        chosen.len() >= max || (chosen.len() >= min && possible.iter().all(|c| chosen.contains(c)))
    };
    // `isChoiceCompleted` holds only once the player was asked (or a card auto-chosen)
    let mut asked = false;
    loop {
        let before = chosen.len();
        loop {
            if asked && complete(&chosen) {
                return Some(chosen);
            }
            let left: Vec<ObjId> = possible.iter().copied().filter(|c| !chosen.contains(c)).collect();
            // `TargetImpl.tryToAutoChoose`: exactly the cards needed
            if min == max && left.len() == min - chosen.len() && !left.is_empty() {
                chosen.push(left[0]);
                asked = true;
                continue;
            }
            asked = true;
            let required = chosen.len() < min && !left.is_empty();
            match ask(game, d, seat, &Query::Cards { objs: left.clone(), required }) {
                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                Answer::Obj(o) if left.contains(&o) => chosen.push(o),
                Answer::Stop => return None,
                Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                _ if chosen.len() >= min || !required => break,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        }
        if chosen.len() == before {
            return Some(chosen);
        }
    }
}

/// `choose_cards`, `cancellable` for a target that isn't required (a cost's, `setRequired(false)`): a boolean ends
/// the choice with fewer than `min` chosen
#[allow(clippy::too_many_arguments)]
fn choose_cards_with(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    shown: &[ObjId],
    possible: &[ObjId],
    min: usize,
    max: usize,
    rule: &str,
    cancellable: bool,
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
            let all = game.player(seat).auto_target_all;
            if let Some(c) = left.iter().copied().find(|c| all || !(feel_bad && owns_target(game, seat, *c))) {
                chosen.push(c);
                continue;
            }
        }
        let required = chosen.len() < min && !left.is_empty();
        match ask(game, d, seat, &Query::Cards { objs: left.clone(), required }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if left.contains(&o) => {
                chosen.push(o);
                // `TargetImpl.isChoiceCompleted`: nothing more to choose
                if chosen.len() >= min && possible.iter().all(|c| chosen.contains(c)) {
                    return Some(chosen);
                }
            }
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ if chosen.len() >= min || !required || cancellable => return Some(chosen),
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}

/// `TargetImpl.choose` (a cost's target, RemoveCounterCost): it asks the player again after each round that added a
/// target and didn't complete the choice, so "done" with a target picked is asked twice
fn choose_in_rounds(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    possible: &[ObjId],
    min: usize,
    max: usize,
) -> Option<Vec<ObjId>> {
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let before = chosen.len();
        // one `HumanPlayer.choose`: picks until the target is complete or "done"
        loop {
            let left: Vec<ObjId> = possible.iter().copied().filter(|c| !chosen.contains(c)).collect();
            let required = chosen.len() < min && !left.is_empty();
            match ask(game, d, seat, &Query::Cards { objs: left.clone(), required }) {
                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                Answer::Obj(o) if left.contains(&o) => {
                    chosen.push(o);
                    if chosen.len() >= max || possible.iter().all(|c| chosen.contains(c)) {
                        return Some(chosen);
                    }
                }
                Answer::Stop => return None,
                Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                _ if !required => break,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        }
        if chosen.len() == before || chosen.len() < min {
            return Some(chosen);
        }
    }
}

/// after the draw step, a lore counter on each of the active player's Sagas
fn add_saga_lore(game: &mut Game, active: Seat) {
    for id in game.battlefield.iter().copied().collect::<Vec<_>>() {
        let o = game.obj(id);
        if o.controller == active && o.def.abilities.iter().any(|a| matches!(a, AbilityDef::SagaAbility { .. })) {
            add_lore(game, id, 1);
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

/// `RoleAssignment.hasSharedRoles` for card types: each card can get a card type of its own (a maximum matching)
fn distinct_card_types(game: &Game, cards: &[ObjId]) -> bool {
    let roles: Vec<Vec<CardType>> = cards.iter().map(|c| game.obj(*c).def.types.clone()).collect();
    fn find(i: usize, roles: &[Vec<CardType>], owners: &mut Vec<(CardType, usize)>, seen: &mut Vec<CardType>) -> bool {
        for r in &roles[i] {
            if seen.contains(r) {
                continue;
            }
            seen.push(*r);
            let owner = owners.iter().find(|(t, _)| t == r).map(|(_, o)| *o);
            if owner.is_none() || find(owner.unwrap(), roles, owners, seen) {
                owners.retain(|(t, _)| t != r);
                owners.push((*r, i));
                return true;
            }
        }
        false
    }
    let mut owners = Vec::new();
    for i in 0..roles.len() {
        find(i, &roles, &mut owners, &mut Vec::new());
    }
    owners.len() >= cards.len()
}

/// Moves a card to the top or bottom of its owner's library.
fn to_library(game: &mut Game, id: ObjId, top: bool) {
    game.move_to(id, Zone::Library);
    // (a token isn't put there: it ceases to exist)
    if top && !game.obj(id).token {
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
            game.entering_together = cards.to_vec();
            let pending_before = game.pending.len();
            for c in cards {
                enter_battlefield(game, d, *c, seat, false);
                if to == P::BattlefieldTapped {
                    game.obj_mut(*c).tapped = true;
                }
            }
            game.entering_together.clear();
            // `GameImpl.moveCards`: the enter events come after they're all in, each in turn, and each one's
            // triggers in Java's order (`trigger_key`)
            if game.pending.len() > pending_before + 1 {
                let mut fired = game.pending.split_off(pending_before);
                let key = |g: &Game, t: &StackItem| {
                    let event = t.entering.unwrap_or(t.card);
                    let at = cards.iter().position(|c| *c == event).unwrap_or(cards.len());
                    (at, g.trigger_key(t))
                };
                fired.sort_by_key(|t| key(game, t));
                game.pending.extend(fired);
            }
            !d.stopped()
        }
    }
}

/// A condition where it's known whether the spell was kicked.
fn condition_with_kicker(game: &Game, c: &Condition, you: Seat, source: ObjId, kicked: bool) -> bool {
    match c {
        // a cast trigger's item carries "five or more mana was spent" as its kicked flag
        Condition::Kicked | Condition::Opus => kicked,
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
    game.player_mut(who).sacrificed += 1;
    note(game, EventKind::Sacrificed, Some(who), None, Some(Target::Obj(id)), 0);
    // what left in the same action still sees it (Java's short-living LKI): sacrificed together
    // and so does what was sacrificed since the player last passed, cast or made mana (activating an ability doesn't
    // reset it)
    let mut sources = game.trigger_sources();
    for o in game.lookback.iter().chain(game.short_lki.iter()) {
        if *o != id && !game.battlefield.contains(o) && !sources.contains(o) {
            sources.push(*o);
        }
    }
    game.short_lki.push(id);
    let fired = game.triggers(sources.clone(), |g, src, you, t| {
        matches!(t, Trigger::Sacrificed { filter, who: w } if Game::whose(w, you, who) && g.filter_matches(filter, you, Some(src), id))
    });
    let batch = game.triggers(sources, |g, src, you, t| {
        matches!(t, Trigger::SacrificedBatch { filter, who: w } if Game::whose(w, you, who) && g.filter_matches(filter, you, Some(src), id))
            && !g.batch_pending.iter().any(|p| p.card == src && same_ability(g, p, t))
    });
    // `PermanentImpl.sacrifice` fires SACRIFICED_PERMANENT at once, while the move's ZONE_CHANGE waits with the
    // simultaneous events: what triggers on the sacrifice comes before what triggers on it leaving
    let at = game.sac_insert.unwrap_or(game.pending.len());
    game.move_to(id, Zone::Graveyard);
    let mut sac = Vec::new();
    for mut t in fired {
        if set_target_pointer(game, &t) {
            t.targets = vec![vec![Target::Player(who)]];
            t.target_zcc = vec![vec![0]];
        }
        sac.push(t);
    }
    let at = at.min(game.pending.len());
    let n = sac.len();
    game.pending.splice(at..at, sac);
    if game.sac_insert.is_some() {
        game.sac_insert = Some(at + n);
    }
    // "one or more": the batch event comes after the others
    game.batch_pending.extend(batch);
}

/// Destroys a permanent, unless it's indestructible.
/// `n` counters put on `o` by `by`'s effect, logged as `CardUtil.addCountersLog`... writes it
fn put_counters(game: &mut Game, o: ObjId, name: &str, n: i32, by: Seat) {
    game.obj_mut(o).add_counters(name, n);
    note(game, EventKind::Counters, Some(by), None, Some(Target::Obj(o)), n);
    if log::kept(game) {
        let mut line = format!(
            "{} puts {} {} counter{} on {}, remaining {}",
            log::player(by),
            n,
            name,
            if n == 1 { "" } else { "s" },
            log::object(game, o),
            game.obj(o).counter(name)
        );
        if let Some(s) = log::source(game).filter(|s| *s != o) {
            line += &format!(" (source: {})", log::object(game, s));
        }
        log::inform(game, line);
    }
}

fn destroy(game: &mut Game, id: ObjId) {
    if game.obj(id).zone == Zone::Battlefield
        && !game.has_ability(id, &AbilityDef::IndestructibleAbility)
        && !shield(game, id)
        && !umbra_armor(game, id)
        && !regenerate(game, id)
    {
        let creature = game.has_type(id, CardType::Creature);
        let token = game.obj(id).token;
        game.move_to(id, Zone::Graveyard);
        if log::kept(game) && !token && game.obj(id).zone == Zone::Graveyard {
            // `PermanentImpl.destroy`
            let mut line = format!("{} {}", log::object(game, id), if creature { "died" } else { "was destroyed" });
            if let Some(s) = log::source(game) {
                line += &format!(" by {}", log::object(game, s));
            }
            log::inform(game, line);
        }
    }
}

/// `UmbraArmorEffect`: an Aura with umbra armor on it is destroyed instead, and its damage removed. True: it was.
fn umbra_armor(game: &mut Game, id: ObjId) -> bool {
    let Some(aura) = game
        .battlefield
        .iter()
        .copied()
        .find(|a| game.attached(*a) == Some(id) && game.has_ability(*a, &AbilityDef::UmbraArmorAbility))
    else {
        return false;
    };
    let o = game.obj_mut(id);
    o.damage = 0;
    o.deathtouched = false;
    destroy(game, aura);
    true
}

/// `ShieldCounterEffect`: a shield counter is removed instead. True: it was.
fn shield(game: &mut Game, id: ObjId) -> bool {
    if game.obj(id).counter("shield") == 0 {
        return false;
    }
    game.obj_mut(id).add_counters("shield", -1);
    true
}

/// Evolve's check: both creatures, and the one that entered has greater power or toughness
fn evolves(game: &Game, src: ObjId, entered: ObjId) -> bool {
    let (sp, st) = game.power_toughness(src);
    let (ep, et) = game.power_toughness(entered);
    game.has_type(src, CardType::Creature) && game.has_type(entered, CardType::Creature) && (ep > sp || et > st)
}

/// `PermanentImpl.removeFromCombat`: no longer attacking or blocking (a creature that stopped being one)
fn remove_from_combat(game: &mut Game, id: ObjId) {
    let (attacking, blocking) = {
        let o = game.obj_mut(id);
        let a = o.attacking.take().is_some();
        o.attacking_pw = None;
        o.blocked = false;
        o.blocks = 0;
        (a, std::mem::take(&mut o.blocking))
    };
    let blockers = std::mem::take(&mut game.obj_mut(id).blocked_by);
    if attacking {
        for b in blockers {
            game.obj_mut(b).blocking.retain(|x| *x != id);
        }
    }
    for a in blocking {
        game.obj_mut(a).blocked_by.retain(|x| *x != id);
    }
}

/// `PermanentImpl.regenerate`, if it has a shield: tapped, removed from combat, its damage removed. True: it did.
fn regenerate(game: &mut Game, id: ObjId) -> bool {
    if game.obj(id).regen_shields == 0 {
        return false;
    }
    tap(game, id);
    let (attacking, blocking) = {
        let o = game.obj_mut(id);
        o.regen_shields -= 1;
        o.damage = 0;
        o.deathtouched = false;
        let a = o.attacking.take().is_some();
        o.attacking_pw = None;
        o.blocked = false;
        o.blocks = 0;
        (a, std::mem::take(&mut o.blocking))
    };
    let blockers = std::mem::take(&mut game.obj_mut(id).blocked_by);
    if attacking {
        for b in blockers {
            game.obj_mut(b).blocking.retain(|x| *x != id);
        }
    }
    for a in blocking {
        game.obj_mut(a).blocked_by.retain(|x| *x != id);
    }
    true
}

/// The permanents of a filter, as `getActivePermanents` lists them (battlefield order).
fn matching(game: &Game, filter: &crate::card::Filter, you: Seat, source: ObjId) -> Vec<ObjId> {
    game.battlefield.iter().copied().filter(|id| game.filter_matches(filter, you, Some(source), *id)).collect()
}

/// An ability until end of turn (the only duration resolving grants have yet).
fn grant(game: &mut Game, target: ObjId, ability: &AbilityDef) {
    let zcc = game.obj(target).zcc;
    Arc::make_mut(&mut game.grants).push(Grant {
        target,
        zcc,
        ability: ability.clone(),
        until_end_of_turn: true,
        until_turn_of: None,
    });
}

/// How many tokens are created for each one an effect would create under `seat`'s control (Elspeth, Storm Slayer)
fn token_copies(game: &Game, seat: Seat) -> usize {
    let mut n = 1;
    game.any_static(|g, src, e| {
        if matches!(e, EffectDef::CreateTwiceThatManyTokensEffect) && g.obj(src).controller == seat {
            n *= 2;
        }
        false
    });
    n
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
/// `MockingbirdEffect` (an optional `EntersBattlefieldAbility`): it may enter as a copy of a creature with mana value
/// up to the mana spent to cast it, a Bird with flying besides
fn enter_as_copy(game: &mut Game, d: &mut dyn Decider, id: ObjId, controller: Seat, cast: bool) {
    // a copy of a Mockingbird has the replacement again, a new one Java hasn't applied yet: it asks again
    loop {
        let def = game.obj(id).def.clone();
        let Some(rule) = def.abilities.iter().find_map(|a| match a {
            AbilityDef::EntersBattlefieldAbility { effects, rule, .. }
                if matches!(effects.as_slice(), [EffectDef::MockingbirdEffect]) =>
            {
                Some(rule.clone())
            }
            _ => None,
        }) else {
            return;
        };
        let q = Query::YesNo { message: format!("Use effect of {}?", def.name) };
        if ask(game, d, controller, &q) != Answer::Bool(true) {
            return;
        }
        let spent = if cast { game.obj(id).mana_spent } else { 0 };
        let possible: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| *o != id && game.has_type(*o, CardType::Creature) && game.obj(*o).def.mana_value() <= spent)
            .collect();
        if possible.is_empty() {
            return;
        }
        let Some(chosen) = choose_cards(game, d, controller, &possible, &possible, 1, 1, &rule) else {
            return;
        };
        let Some(of) = chosen.first().copied() else {
            return;
        };
        let o = game.obj(of);
        let mut copy = (*o.copiable()).clone();
        // (Java's `addSubType` keeps a creature type off what isn't a creature: a copy of an unanimated Mutavault)
        if copy.types.contains(&CardType::Creature) && !copy.subtypes.iter().any(|t| t == "Bird") {
            copy.subtypes.push("Bird".into());
        }
        if !copy.abilities.contains(&AbilityDef::FlyingAbility) {
            copy.abilities.push(AbilityDef::FlyingAbility);
        }
        let copy = Arc::new(copy);
        let o = game.obj_mut(id);
        if o.printed_def.is_none() {
            o.printed_def = Some(def);
        }
        o.copied_def = Some(copy.clone());
        o.def = copy;
    }
}

fn enter_battlefield(game: &mut Game, d: &mut dyn Decider, id: ObjId, controller: Seat, kicked: bool) {
    let (from, zcc) = (game.obj(id).zone, game.obj(id).zcc);
    let source = game.resolving.map(|(_, card)| card).filter(|c| *c != id);
    enter_battlefield_now(game, d, id, controller, kicked);
    if log::kept(game) && game.obj(id).token && game.obj(id).zone == Zone::Battlefield {
        // `TokenImpl.putOntoBattlefield`
        let line = format!("{} creates a {} token", log::player(controller), log::object(game, id));
        log::inform(game, line);
    } else if log::kept(game) && game.obj(id).zone == Zone::Battlefield && game.obj(id).zcc != zcc {
        // `PlayerImpl.moveCards`, with what put it there (`CardUtil.getSourceLogName`)
        let mut line = format!(
            "{} puts {} from {} onto the Battlefield",
            log::player(controller),
            log::object(game, id),
            log::zone(from)
        );
        if let Some(src) = source {
            line += &format!(" (source: {})", log::object(game, src));
        }
        log::inform(game, line);
    }
}

/// A card put onto the battlefield from outside the game (the game host's practice tools, Java's
/// `PracticeTools.addCards`): it enters as any permanent does, its "enters" abilities and triggers with it
pub fn put_onto_battlefield(game: &mut Game, d: &mut dyn Decider, id: ObjId, controller: Seat) {
    enter_battlefield(game, d, id, controller, false);
}

/// where double-faced permanents' triggers start in the order (`Game::dfc_registered`)
const DFC_ORDER: u64 = 1 << 24;

fn enter_battlefield_now(game: &mut Game, d: &mut dyn Decider, id: ObjId, controller: Seat, kicked: bool) {
    // a token of the class last made takes its image
    if game.obj(id).token && game.obj(id).token_image.is_none() {
        if let Some((name, class, set, pick)) = game.token_made.clone() {
            if name == game.obj(id).def.name {
                game.obj_mut(id).token_image = Some((class, set, pick));
            }
        }
    }
    // Weathered Runestone: nonland permanent cards in graveyards and libraries can't enter (a token isn't a card:
    // Java's `game.getCard` doesn't find it)
    if matches!(game.obj(id).zone, Zone::Graveyard | Zone::Library)
        && !game.obj(id).token
        && !game.obj(id).is(CardType::Land)
        && game.any_static(|_, _, e| matches!(e, EffectDef::WeatheredRunestoneEffect))
    {
        return;
    }
    // a permanent spell resolving was cast (`CastFromEverywhereSourceCondition`)
    let cast = game.obj(id).zone == Zone::Stack;
    let stack_zcc = game.obj(id).zcc;
    // each its own replacement effect (Kumano's "enters with an additional +1/+1 counter")
    let mut counter_effects = 0;
    let more_counters: i32 = if cast {
        let (mine, rest): (Vec<_>, Vec<_>) =
            std::mem::take(&mut game.enter_counters).into_iter().partition(|(o, z, _)| *o == id && *z == stack_zcc);
        game.enter_counters = rest;
        counter_effects = mine.len();
        mine.iter().map(|(_, _, n)| n).sum()
    } else {
        0
    };
    // Giada: each other Angel you control enters with a +1/+1 counter for each Angel you already control (not one
    // entering with it: Collected Company's two). Java moves cards together one at a time and an entering
    // permanent's static abilities already apply (`AbilityImpl.isInUseableZone`), so a Giada moved before this
    // Angel gives it counters, one moved after doesn't
    let mut more_counters = more_counters;
    // Giadas whose replacement effect applies (`GiadaFontOfHopeEntersBattlefieldEffect.applies`), counters or not
    let mut giada_effects = 0;
    if game.obj(id).def.subtypes.iter().any(|t| t == "Angel") {
        let together = |o: &ObjId| game.entering_together.contains(o);
        let mine = game.entering_together.iter().position(|o| *o == id);
        let before: Vec<ObjId> = match mine {
            Some(i) => game.entering_together[..i].to_vec(),
            None => Vec::new(),
        };
        let angels = game
            .battlefield
            .iter()
            .filter(|o| !together(o) && game.obj(**o).controller == controller && game.has_subtype(**o, "Angel"))
            .count() as i32;
        let giadas =
            game.battlefield.iter().chain(before.iter().filter(|o| !game.battlefield.contains(o))).filter(|src| {
                **src != id
                    && (!together(src) || before.contains(src))
                    && game.obj(**src).controller == controller
                    && game.obj(**src).def.abilities.iter().any(|a| {
                        matches!(a, AbilityDef::SimpleStaticAbility { effects, .. }
                        if effects.contains(&EffectDef::GiadaFontOfHopeEntersBattlefieldEffect))
                    })
            });
        giada_effects = giadas.count();
        more_counters += giada_effects as i32 * angels;
    }
    // the characteristics its replacement effects see (`Game::enters_as`)
    let entering_as = game.enters_as.clone().filter(|(o, _)| *o == id).map(|(_, def)| def);
    let real_def = entering_as.as_ref().map(|def| std::mem::replace(&mut game.obj_mut(id).def, def.clone()));
    // replacement effects on its entering, more than one: the player picks the order (`chooseReplacementEffect`):
    // Gev's counters for another creature, the next creature spell's counters, and the creature's own "enters"
    // abilities (`EntersBattlefieldEffect`s: one whether its condition holds or not, an `EscapesWithAbility` whether
    // it escaped or not)
    if game.obj(id).def.is(CardType::Creature) {
        let gevs = game
            .battlefield
            .iter()
            .filter(|src| {
                **src != id
                    && game.obj(**src).controller == controller
                    && game.obj(**src).def.abilities.iter().any(|a| {
                        matches!(a, AbilityDef::SimpleStaticAbility { effects, .. }
                            if effects.contains(&EffectDef::GevScaledScorchEntersEffect))
                    })
            })
            .count();
        let own = game
            .obj(id)
            .def
            .abilities
            .iter()
            .filter(|a| match a {
                // `EntersBattlefieldEffect.applies` checks a kicker condition (Gnarlid Colony unkicked has none)
                AbilityDef::EntersBattlefieldAbility { condition: Some(Condition::Kicked), .. } => kicked,
                _ => matches!(
                    a,
                    AbilityDef::EntersPreparedAbility { .. }
                        | AbilityDef::EscapesWithAbility { .. }
                        | AbilityDef::EntersBattlefieldAbility { .. }
                        | AbilityDef::EntersBattlefieldTappedAbility { .. }
                        | AbilityDef::AsEntersBattlefieldAbility { .. }
                ),
            })
            .count();
        // "creatures your opponents control enter tapped" (Authority of the Consuls)
        let was = game.obj(id).controller;
        game.obj_mut(id).controller = controller;
        let tappers = game
            .battlefield
            .iter()
            .flat_map(|src| {
                game.obj(*src).def.abilities.iter().filter_map(move |a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => Some((*src, effects)),
                    _ => None,
                })
            })
            .filter(|(src, effects)| {
                effects.iter().any(|e| {
                    matches!(e, EffectDef::PermanentsEnterBattlefieldTappedEffect { filter }
                        if game.filter_matches(filter, game.obj(*src).controller, Some(*src), id))
                })
            })
            .count();
        game.obj_mut(id).controller = was;
        let others = gevs + counter_effects + tappers + giada_effects;
        if others > 0 {
            for _ in 1..others + own {
                game.owed_choices.push(controller);
            }
        }
    }
    if let Some(def) = real_def.clone() {
        game.obj_mut(id).def = def;
    }
    // a double-faced card's triggers: Java registers its halves as it loads, but the permanent's under new keys as it
    // first enters, so they wait after every other card's (`TriggeredAbilities` is a LinkedHashMap)
    {
        let o = game.obj(id);
        let dfc = o.printed_def.as_ref().unwrap_or(&o.def);
        if !o.token && (dfc.transform_back.is_some() || dfc.back.is_some()) && o.trigger_order < DFC_ORDER {
            let k = DFC_ORDER + game.dfc_registered;
            game.dfc_registered += 1;
            game.obj_mut(id).trigger_order = k;
        }
    }
    let turn = game.turn;
    let owner = game.obj(id).owner;
    let from_hand = game.player(owner).hand.iter().position(|x| *x == id);
    game.entering_from_hand = from_hand.map(|i| (id, owner, i));
    game.move_to(id, Zone::Battlefield);
    if game.obj(id).is(CardType::Land) {
        game.player_mut(controller).lands_entered += 1;
    }
    enter_as_copy(game, d, id, controller, cast);
    // `EntersPreparedAbility`
    if game.obj(id).def.abilities.iter().any(|a| matches!(a, AbilityDef::EntersPreparedAbility { .. })) {
        become_prepared(game, id);
    }
    // a Room: its doors locked, then the cast half's unlocked (`RoomEnterUnlockEffect`)
    if !game.obj(id).def.room.is_empty() {
        let full = game.obj(id).def.clone();
        let o = game.obj_mut(id);
        o.doors = [false; 2];
        if o.printed_def.is_none() {
            o.printed_def = Some(full);
        }
        let half = o.room_cast.take();
        set_room_face(game, id);
        if let Some(k) = half {
            unlock_door(game, id, k);
        }
    }
    // at night a daybound card still enters as its front (Java never calls `NightboundAbility.checkCard`); the
    // state-based check transforms it
    let def = game.obj(id).def.clone();
    game.obj_mut(id).controller = controller;
    if let Some(def) = entering_as.clone() {
        game.obj_mut(id).def = def;
    }
    let tapped_by_static = game.any_static(|g, src, e| match e {
        EffectDef::PermanentsEnterBattlefieldTappedEffect { filter } => {
            g.filter_matches(filter, g.obj(src).controller, Some(src), id)
        }
        _ => false,
    });
    if let Some(def) = real_def {
        game.obj_mut(id).def = def;
    }
    let zcc = {
        let o = game.obj_mut(id);
        o.controller = controller;
        if tapped_by_static || def.abilities.iter().any(|a| a.is_enters_tapped()) {
            o.tapped = true;
        }
        if let Some(n) = def.loyalty.filter(|n| *n > 0) {
            o.add_counters("loyalty", n);
        }
        if def.is(CardType::Battle) {
            if let Some(n) = def.defense.filter(|n| *n > 0) {
                o.add_counters("defense", n);
            }
        }
        o.kicked = kicked;
        o.cast = cast;
        o.entered_turn = turn;
        // `EscapesWithAbility`
        if std::mem::take(&mut o.escaped) {
            for a in &def.abilities {
                if let AbilityDef::EscapesWithAbility { counters, .. } = a {
                    o.add_counters("+1/+1", *counters);
                }
            }
        }
        o.zcc
    };
    // a battle: its protector, the one opponent (`chooseProtector` asks only with more than one)
    if def.is(CardType::Battle) {
        let opponent = game.opponent(controller);
        game.obj_mut(id).protector = Some(opponent);
    }
    // blitz: haste, and sacrificed at the beginning of the next end step
    if game.obj(id).blitzed {
        Arc::make_mut(&mut game.grants).push(Grant {
            target: id,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: false,
            until_turn_of: None,
        });
        let delayed = Delayed {
            rule: "At the beginning of the next end step, sacrifice the blitzed creature.".into(),
            sacrifice: vec![(id, zcc)],
            warp_exile: Vec::new(),
            effects: Vec::new(),
            source_zcc: 0,
            fixed: Vec::new(),
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, controller) });
    }
    // dash: haste, and returned to its owner's hand at the beginning of the next end step
    if game.obj(id).dashed {
        Arc::make_mut(&mut game.grants).push(Grant {
            target: id,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: false,
            until_turn_of: None,
        });
        let delayed = Delayed {
            rule: "At the beginning of the next end step, return the dashed creature from the battlefield to its owner's hand.".into(),
            sacrifice: Vec::new(),
            warp_exile: Vec::new(),
            effects: vec![EffectDef::ReturnToHandTargetEffect],
            source_zcc: zcc,
            fixed: vec![(id, zcc)],
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, controller) });
    }
    // `WarpAbilityWatcher`: a warped permanent is exiled at the beginning of the next end step
    if game.obj(id).warped {
        let delayed = Delayed {
            rule: "At the beginning of the next end step, exile this creature if it was cast for its warp cost.".into(),
            sacrifice: Vec::new(),
            warp_exile: vec![(id, zcc)],
            effects: Vec::new(),
            source_zcc: 0,
            fixed: Vec::new(),
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, controller) });
    }
    if more_counters > 0 {
        game.obj_mut(id).add_counters("+1/+1", more_counters);
    }
    // sunburst: a counter for each color of mana spent to cast it
    if cast && game.obj(id).def.has(&AbilityDef::SunburstAbility) {
        let n = game.obj(id).spent_colors.iter().filter(|k| **k > 0).count() as i32;
        if n > 0 {
            let kind = if game.has_type(id, CardType::Creature) { "+1/+1" } else { "charge" };
            game.obj_mut(id).add_counters(kind, n);
        }
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
            // `TargetImpl.tryToAutoChoose` reads the source ability's rule for its feel-bad words (Old-Growth Troll's
            // "Sacrifice this land" keeps its one Forest asked)
            let rule = match game.resolving_item.clone() {
                Some(item) => crate::view::item_rule(game, &item),
                None => format!("attach {} to", def.name),
            };
            if let Some(chosen) = choose_target(game, d, controller, t, Some(id), &rule, false) {
                if let Some(Target::Obj(to)) = chosen.first().copied() {
                    let zcc = game.obj(to).zcc;
                    game.attach_to(id, to, zcc);
                }
            }
        }
    }
    // shock lands: `TapSourceUnlessPaysEffect` asks only when the life can be paid
    for a in &def.abilities {
        if let AbilityDef::AsEntersBattlefieldAbility { effects, .. } = a {
            for e in effects {
                // Pithing Needle: "as this enters, choose a card name"
                if let EffectDef::ChooseLandNameEffect = e {
                    let q = Query::Choice { message: "Card name".into(), choices: Vec::new() };
                    let chosen = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Text(t) => break t,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    game.obj_mut(id).chosen_name = Some(chosen);
                }
                if let EffectDef::ChooseCreatureEffect { filter } = e {
                    let def = TargetDef::TargetPermanent { filter: filter.clone(), min: 1, max: 1, distribute: None };
                    // `chooseTarget` with nothing to choose returns without asking
                    let chosen = if target_possible(game, controller, &def, Some(id)) {
                        choose_target(game, d, controller, &def, Some(id), "", false)
                    } else {
                        None
                    };
                    if d.stopped() {
                        return;
                    }
                    game.obj_mut(id).chosen = match chosen.as_ref().and_then(|c| c.first()) {
                        Some(Target::Obj(o)) => Some((*o, game.obj(*o).zcc)),
                        _ => None,
                    };
                }
                if let EffectDef::ChooseCreatureTypeEffect = e {
                    let q = Query::Choice { message: "Choose a creature type".into(), choices: Vec::new() };
                    let chosen = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Text(t) => break t,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    game.obj_mut(id).add_info("chosen type", &format!("Chosen type: {}", chosen));
                    game.obj_mut(id).chosen_type = Some(chosen);
                }
                if let EffectDef::EntersTappedChooseColorEffect = e {
                    game.obj_mut(id).tapped = true;
                    let names = ["White", "Blue", "Black", "Red", "Green"];
                    let choices: Vec<String> = names.iter().map(|n| n.to_string()).collect();
                    let q = Query::Choice { message: "Choose color".into(), choices };
                    let c = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Text(t) => {
                                if let Some(k) = names.iter().position(|n| *n == t) {
                                    break Color::ALL[k];
                                }
                            }
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    let mut land = (*game.obj(id).def).clone();
                    for a in land.abilities.iter_mut() {
                        if let AbilityDef::ChosenColorManaAbility { amount, land_abilities, rule } = a {
                            *a = if *land_abilities {
                                AbilityDef::ConditionalManaAbility {
                                    color: Some(c),
                                    amount: *amount,
                                    combination: false,
                                    filter: crate::card::Filter::default(),
                                    chosen_type: false,
                                    cant_counter: false,
                                    cost: Vec::new(),
                                    land_abilities: true,
                                    creature_abilities: false,
                                    colorless: false,
                                    filter_abilities: false,
                                    rule: rule.clone(),
                                }
                            } else {
                                AbilityDef::ColoredManaAbility {
                                    color: c,
                                    amount: *amount,
                                    cost: Vec::new(),
                                    no_tap: false,
                                    creature_spell_haste: false,
                                    net: None,
                                    rule: rule.clone(),
                                }
                            };
                        }
                    }
                    let o = game.obj_mut(id);
                    if o.printed_def.is_none() {
                        o.printed_def = Some(o.def.clone());
                    }
                    o.def = Arc::new(land);
                    o.add_info(
                        "chosen color",
                        &format!("Chosen color: {}", names[Color::ALL.iter().position(|x| *x == c).unwrap_or(0)]),
                    );
                }
                if let EffectDef::ChooseBasicLandTypeEffect = e {
                    let choices: Vec<String> =
                        ["Forest", "Island", "Mountain", "Plains", "Swamp"].iter().map(|c| c.to_string()).collect();
                    let q = Query::Choice { message: "Choose a basic land type".into(), choices: choices.clone() };
                    let chosen = loop {
                        match ask(game, d, controller, &q) {
                            Answer::Text(t) if choices.contains(&t) => break t,
                            Answer::Stop => return,
                            _ => {}
                        }
                    };
                    game.obj_mut(id).add_info("chosen color", &format!("Chosen basic land type: {}", chosen));
                    // "This land is the chosen type": the subtype and its mana ability, after the printed ones
                    let passage = def.abilities.iter().any(|a| {
                        matches!(a, AbilityDef::SimpleStaticAbility { effects, .. }
                            if effects.contains(&EffectDef::MultiversalPassagePassageEffect))
                    });
                    if passage {
                        let mut land = (*game.obj(id).def).clone();
                        land.subtypes.push(chosen.clone());
                        let (effects, cost) = (Vec::new(), Vec::new());
                        land.abilities.push(match chosen.as_str() {
                            "Plains" => AbilityDef::WhiteManaAbility { effects, cost },
                            "Island" => AbilityDef::BlueManaAbility { effects, cost },
                            "Swamp" => AbilityDef::BlackManaAbility { effects, cost },
                            "Mountain" => AbilityDef::RedManaAbility { effects, cost },
                            _ => AbilityDef::GreenManaAbility { effects, cost },
                        });
                        let o = game.obj_mut(id);
                        if o.printed_def.is_none() {
                            o.printed_def = Some(o.def.clone());
                        }
                        o.def = Arc::new(land);
                    }
                }
                if let EffectDef::TapSourceUnlessBeholdEffect { subtype } = e {
                    let (perms, cards) = beholdable(game, controller, subtype);
                    let paid = !(perms.is_empty() && cards.is_empty()) && {
                        let article = if "AEIOU".contains(&subtype[..1]) { "an" } else { "a" };
                        let q = Query::YesNo {
                            message: format!("behold {} {}? (otherwise {} becomes tapped)", article, subtype, def.name),
                        };
                        loop {
                            match ask(game, d, controller, &q) {
                                Answer::Bool(b) => break b,
                                Answer::Stop => return,
                                _ => {}
                            }
                        }
                    };
                    let cost = CostDef::BeholdCost { subtype: subtype.clone() };
                    if !(paid && pay_cost(game, d, controller, id, &cost)) {
                        if d.stopped() {
                            return;
                        }
                        game.obj_mut(id).tapped = true;
                    }
                }
                // `TapSourceUnlessPaysEffect` with a `RevealTargetFromHandCost`: asked only when a card can be revealed
                if let EffectDef::TapSourceUnlessRevealsEffect { filter, message } = e {
                    let cost = CostDef::RevealTargetFromHandCost { filter: filter.clone(), min: 1, max: 1 };
                    let can = game
                        .player(controller)
                        .hand
                        .iter()
                        .any(|c| *c != id && game.filter_matches(filter, controller, Some(id), *c));
                    let paid = can && {
                        let q = Query::YesNo {
                            message: format!("reveal {}? (otherwise {} becomes tapped)", message, def.name),
                        };
                        loop {
                            match ask(game, d, controller, &q) {
                                Answer::Bool(b) => break b,
                                Answer::Stop => return,
                                _ => {}
                            }
                        }
                    };
                    if !(paid && pay_cost(game, d, controller, id, &cost)) {
                        if d.stopped() {
                            return;
                        }
                        game.obj_mut(id).tapped = true;
                    }
                }
                if let EffectDef::TapSourceUnlessPaysEffect { life } = e {
                    let paid = game.player(controller).life >= *life && {
                        let q = Query::YesNo {
                            message: format!("pay {} life? (otherwise {} becomes tapped)", life, def.name),
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
                        lose_life_by(game, controller, *life, Some(id), false);
                    } else {
                        game.obj_mut(id).tapped = true;
                    }
                }
            }
        }
    }
    game.entering_from_hand = None;
    for a in &def.abilities {
        if let AbilityDef::EntersBattlefieldAbility { effects, condition, .. } = a {
            // Java checks it as the permanent enters (`EntersBattlefieldEffect`, a replacement): it isn't on the
            // battlefield yet, so it doesn't count itself (Hive of the Eye Tyrant's "two or more other lands")
            let holds = condition.as_ref().is_none_or(|c| {
                let at = game.battlefield.iter().position(|x| *x == id);
                if let Some(i) = at {
                    Arc::make_mut(&mut game.battlefield).remove(i);
                }
                let holds = condition_with_kicker(game, c, controller, id, kicked);
                if let Some(i) = at {
                    Arc::make_mut(&mut game.battlefield).insert(i, id);
                }
                holds
            });
            if holds {
                let mut item = StackItem::spell(id, controller);
                item.x = game.obj(id).cast_x;
                apply_effects(game, d, &item, effects, &[], (id, zcc));
            }
        }
    }
    // Java's continuous effects apply before the event is checked: Kaito, entering on its controller's turn with
    // loyalty, enters as a creature
    refresh_becomes(game);
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
        Trigger::Enters
        | Trigger::EntersOrDies
        | Trigger::EntersOrLeaves
        | Trigger::EntersOrAttacks
        | Trigger::TransformsOrEnters
        | Trigger::EntersOrUpkeep => src == id && g.obj(src).zone == Zone::Battlefield,
        Trigger::EntersAll { filter, include_self, graveyard, .. } => {
            *graveyard == (g.obj(src).zone == Zone::Graveyard)
                && ((src == id && *include_self) || g.filter_matches(filter, you, Some(src), id))
        }
        Trigger::PeltCollector => src != id && g.obj(src).zone == Zone::Battlefield && controller == you,
        Trigger::Evolve => g.obj(src).zone == Zone::Battlefield && controller == you && evolves(g, src, id),
        _ => false,
    });
    let mut fired = game.pelt_collector(fired, id);
    // "one or more … enter": once per batch, after the batch's other triggers
    let (batch, rest): (Vec<StackItem>, Vec<StackItem>) =
        fired.into_iter().partition(|t| matches!(game.trigger_of(t), Some(Trigger::EntersAll { batch: true, .. })));
    fired = rest;
    for t in batch {
        if !game.batch_pending.iter().any(|b| b.card == t.card && b.ability == t.ability) {
            game.batch_pending.push(t);
        }
    }
    for t in fired.iter_mut() {
        if matches!(game.trigger_of(t), Some(Trigger::Evolve | Trigger::EntersAll { .. })) {
            t.entering = Some(id);
        }
    }
    let cast_x = game.obj(id).cast_x;
    for t in fired.iter_mut() {
        if t.card == id {
            t.x = cast_x;
        }
        let a = t.ability.and_then(|(i, _)| game.item_def(t).abilities.get(i).cloned());
        if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. })) {
            t.targets = vec![vec![Target::Obj(id)]];
            t.target_zcc = vec![vec![zcc]];
        }
        if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. })) {
            t.targets = vec![vec![Target::Player(controller)]];
            t.target_zcc = vec![vec![0]];
        }
    }
    // entering at the same time as cards put before it: its "whenever another ... enters" sees them enter too
    let earlier: Vec<ObjId> = game
        .entering_together
        .iter()
        .copied()
        .take_while(|e| *e != id)
        .filter(|e| game.battlefield.contains(e))
        .collect();
    for e in earlier {
        let mut more = game.triggers([id], |g, src, you, t| match t {
            Trigger::EntersAll { filter, graveyard: false, .. } => {
                g.obj(src).zone == Zone::Battlefield && g.filter_matches(filter, you, Some(src), e)
            }
            _ => false,
        });
        for t in more.iter_mut() {
            t.entering = Some(e);
            let a = t.ability.and_then(|(i, _)| game.item_def(t).abilities.get(i));
            if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. })) {
                t.targets = vec![vec![Target::Obj(e)]];
                t.target_zcc = vec![vec![game.obj(e).zcc]];
            }
        }
        for t in more {
            if matches!(game.trigger_of(&t), Some(Trigger::EntersAll { batch: true, .. })) {
                if !game.batch_pending.iter().any(|b| b.card == t.card && b.ability == t.ability) {
                    game.batch_pending.push(t);
                }
            } else {
                fired.push(t);
            }
        }
    }
    game.pending.extend(fired);
}

/// Lore counters on a Saga: its chapter abilities whose chapter they reach trigger
fn add_lore(game: &mut Game, id: ObjId, n: i32) {
    let before = game.obj(id).counter("lore");
    game.obj_mut(id).add_counters("lore", n);
    let after = before + n;
    let fired = game
        .triggers([id], |_, _, _, t| matches!(t, Trigger::Chapter { n } if before < *n as i32 && *n as i32 <= after));
    game.pending.extend(fired);
}

/// Behold: the permanents of a subtype `seat` controls and the cards of it in their hand
fn beholdable(game: &Game, seat: Seat, subtype: &str) -> (Vec<ObjId>, Vec<ObjId>) {
    let perms = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| game.obj(*b).controller == seat && game.has_subtype(*b, subtype))
        .collect();
    let cards = game
        .player(seat)
        .hand
        .iter()
        .copied()
        .filter(|c| game.obj(*c).def.subtypes.iter().any(|s| s == subtype))
        .collect();
    (perms, cards)
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
/// `DAMAGED_BATCH_FOR_ONE_PERMANENT`: each permanent dealt damage since the last check, with the total
fn damaged_batch_triggers(game: &mut Game) {
    let batch = std::mem::take(&mut game.damaged_batch);
    let mut totals: Vec<(ObjId, i32)> = Vec::new();
    for (o, n) in batch {
        match totals.iter_mut().find(|(x, _)| *x == o) {
            Some(t) => t.1 += n,
            None => totals.push((o, n)),
        }
    }
    for (o, n) in totals {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        for mut t in game.triggers([o], |_, _, _, t| matches!(t, Trigger::DealtDamage)) {
            t.x = n;
            game.pending.push(t);
        }
    }
}

/// "Whenever you put one or more +1/+1 counters on this" (`COUNTERS_ADDED`), for the permanents that got some
fn counters_added_triggers(game: &mut Game) {
    let got: Vec<ObjId> = game.battlefield.iter().copied().filter(|b| game.obj(*b).p1p1_added).collect();
    for o in game.objs_with_p1p1_added() {
        game.obj_mut(o).p1p1_added = false;
    }
    for o in got {
        let fired = game
            .triggers([o], |_, _, _, t| matches!(t, Trigger::CountersAddedSource { counter } if counter == "+1/+1"));
        game.pending.extend(fired);
    }
}

/// `StateTriggeredAbility`: it triggers while its state holds, once until it has resolved (or left the stack)
fn state_triggers(game: &mut Game) {
    let sources: Vec<ObjId> = game.battlefield.to_vec();
    let fired = game.triggers(sources, |g, src, _, t| match t {
        Trigger::SourceCountersAtLeast { counter, n } => g.obj(src).counter(counter) >= *n,
        _ => false,
    });
    for t in fired {
        let waiting = game.pending.iter().chain(game.stack.iter()).any(|p| p.card == t.card && p.ability == t.ability);
        if !waiting {
            game.pending.push(t);
        }
    }
}

/// True: state-based actions were performed or triggers were put on the stack (`GameImpl.checkStateAndTriggered`).
fn check_state_and_triggered(game: &mut Game, d: &mut dyn Decider) -> bool {
    let mut happened = false;
    loop {
        if !game.batch_pending.is_empty() {
            let batch = std::mem::take(&mut game.batch_pending);
            game.pending.extend(batch);
        }
        if game.exiled_batch {
            game.exiled_batch = false;
            let active = game.active.unwrap_or(0);
            let sources = game.trigger_sources();
            let fired = game.triggers(sources, |_, _, you, t| {
                matches!(t, Trigger::ExiledFromGraveyardOrBattlefieldBatch) && you == active
            });
            game.pending.extend(fired);
        }
        if !game.life_batch.is_empty() {
            // `LOST_LIFE_BATCH_FOR_ONE_PLAYER` fires after the effect: a source that left the battlefield meanwhile
            // (Feed the Swarm's life loss, then its destroy) doesn't trigger
            let batch = std::mem::take(&mut game.life_batch);
            let sources = game.trigger_sources();
            let live: Vec<StackItem> = batch
                .into_iter()
                .map(|(_, t)| t)
                .filter(|t| sources.contains(&t.card) && t.ability.is_none_or(|(_, zcc)| game.obj(t.card).zcc == zcc))
                .collect();
            game.pending.extend(live);
        }
        if !game.damaged_batch.is_empty() {
            damaged_batch_triggers(game);
        }
        counters_added_triggers(game);
        game.update_guildpact();
        happened |= unattach_equipment(game);
        // what the last triggers' targets triggered, now that they're all on the stack
        if let Some(later) = game.targeted_later.take() {
            game.pending.extend(later);
        }
        happened |= state_based_actions(game);
        state_triggers(game);
        if game.over || d.stopped() {
            return happened;
        }
        if legend_rule(game, d) {
            happened = true;
            continue;
        }
        if d.stopped() || (game.pending.is_empty() && game.returning.is_empty()) {
            return happened;
        }
        // something triggered: `checkStateAndTriggered` processes the action, unless none of it was played (each
        // one removed for want of targets: `checkTriggered` is false, and the state isn't checked again)
        let mut played = false;
        game.short_lki.clear();
        let pending = std::mem::take(&mut game.pending);
        let mut pending = waiting(game, pending);
        let active = game.active.unwrap_or(0);
        game.targeted_later = Some(Vec::new());
        for k in 0..game.seats() {
            let seat = (active + k) % game.seats();
            // `GameImpl.checkTriggered` fetches each player's list as their turn comes: what triggered since
            let arrived: Vec<StackItem> = take_seat(game, seat);
            pending.extend(waiting(game, arrived));
            let mut mine: Vec<StackItem> = pending.iter().filter(|t| t.controller == seat).cloned().collect();
            // their triggers that use no stack first: cards coming back from exile. What those trigger waits for the
            // next pass if nothing else of theirs was waiting, and otherwise joins their list after the first pick
            let mut back: Vec<ObjId> = game.returning.iter().filter(|(s, _)| *s == seat).map(|(_, c)| *c).collect();
            let mut refetch = false;
            if !back.is_empty() {
                played = true;
                game.returning.retain(|(s, _)| *s != seat);
                // `AuraReplacementEffect` puts an Aura onto the battlefield as its move is replaced, before the rest
                back.sort_by_key(|c| !game.obj(*c).def.has_subtype("Aura"));
                // they come back at once: each sees the others enter
                game.entering_together = back.clone();
                let pending_before = game.pending.len();
                for c in back.clone() {
                    // a token that left the battlefield has ceased to exist
                    if game.obj(c).zone == Zone::Exile && !game.obj(c).token {
                        if game.return_to_hand.contains(&c) {
                            game.return_to_hand.retain(|x| *x != c);
                            game.move_to(c, Zone::Hand);
                            continue;
                        }
                        let owner = game.obj(c).owner;
                        enter_battlefield(game, d, c, owner, false);
                    }
                }
                game.entering_together.clear();
                // `GameImpl.moveCards`: the enter events come after they're all back, each in turn, and each one's
                // triggers in Java's order (`trigger_key`)
                if game.pending.len() > pending_before + 1 {
                    let mut fired = game.pending.split_off(pending_before);
                    let key = |g: &Game, t: &StackItem| {
                        let event = t.entering.unwrap_or(t.card);
                        let at = back.iter().position(|c| *c == event).unwrap_or(back.len());
                        (at, g.trigger_key(t))
                    };
                    fired.sort_by_key(|t| key(game, t));
                    game.pending.extend(fired);
                }
                if d.stopped() {
                    return happened;
                }
                refetch = !mine.is_empty();
            }
            let mut picked = 0;
            loop {
                if refetch && picked > 0 {
                    let arrived = take_seat(game, seat);
                    mine.extend(waiting(game, arrived));
                    refetch = false;
                }
                if mine.is_empty() {
                    break;
                }
                // the player orders their triggers, unless they're all the same (auto-order)
                let pick = if mine.len() > 1 && !mine.iter().all(|t| same_trigger(game, t, &mine[0])) {
                    let triggers = mine.iter().map(|t| (t.card, crate::view::item_rule(game, t))).collect();
                    match ask(game, d, seat, &Query::PickTrigger { count: mine.len(), triggers }) {
                        Answer::Trigger(i) if i < mine.len() => i,
                        Answer::Stop => return happened,
                        _ => continue,
                    }
                } else {
                    0
                };
                let mut t = mine.remove(pick);
                picked += 1;
                let ability = t.ability.and_then(|(i, _)| game.item_def(&t).abilities.get(i).cloned());
                // a modal one: `HumanPlayer.chooseMode` among the modes whose targets can be chosen
                let modes = ability.as_ref().map(|a| a.trigger_modes()).unwrap_or_default();
                let mut mode_targets = None;
                if !modes.is_empty() {
                    let available: Vec<usize> = (0..modes.len())
                        .filter(|k| modes[*k].1.iter().all(|x| target_possible(game, seat, x, Some(t.card))))
                        .collect();
                    let mut asked = 0;
                    let k = loop {
                        // asked even with one mode left, and again after Cancel or any other answer: a triggered
                        // ability can't be skipped (a decider that never picks one gives up after a while)
                        asked += 1;
                        if available.is_empty() || asked > 100 {
                            break None;
                        }
                        game.moding = Some(t.clone());
                        let answer = ask(game, d, seat, &Query::ChooseMode { modes: available.clone() });
                        game.moding = None;
                        match answer {
                            Answer::Mode(k) if available.contains(&k) => break Some(k),
                            Answer::Stop => return happened,
                            _ => {}
                        }
                    };
                    let Some(k) = k else { continue };
                    t.modes = vec![k];
                    mode_targets = Some(modes[k].1.to_vec());
                }
                let reflexive = t
                    .delayed
                    .as_ref()
                    .filter(|dl| !dl.targets.is_empty())
                    .map(|dl| (dl.rule.clone(), dl.targets.clone()));
                Arc::make_mut(&mut game.stack).push(t);
                let at = game.stack.len();
                // a reflexive trigger's targets, the same way
                if let Some((rule, targets)) = reflexive {
                    if !choose_targets(game, d, seat, &targets, None, &rule, false) {
                        if d.stopped() {
                            return happened;
                        }
                        Arc::make_mut(&mut game.stack).pop();
                    }
                }
                // its targets, which can't be cancelled; with no legal ones it's removed (rule 603.3d)
                if let Some((rule, _, targets, _)) = ability.as_ref().and_then(|a| a.triggered()) {
                    let targets = mode_targets.as_deref().unwrap_or(targets);
                    if !targets.is_empty() {
                        if !choose_targets(game, d, seat, targets, None, rule, false) {
                            if d.stopped() {
                                return happened;
                            }
                            Arc::make_mut(&mut game.stack).pop();
                        } else {
                            // `TRIGGERED_ABILITY`
                            commit_crime(game, seat);
                        }
                    }
                }
                if game.stack.len() == at {
                    played = true;
                }
                if log::kept(game) && game.stack.len() == at {
                    let line = log::triggers(game, seat, &game.stack[at - 1]);
                    log::inform(game, line);
                }
            }
        }
        if !played {
            return happened;
        }
        happened = true;
    }
}

/// Takes `seat`'s waiting triggers out of `game.pending`.
fn take_seat(game: &mut Game, seat: Seat) -> Vec<StackItem> {
    let (mine, rest): (Vec<StackItem>, Vec<StackItem>) =
        std::mem::take(&mut game.pending).into_iter().partition(|t| t.controller == seat);
    game.pending = rest;
    mine
}

/// The triggers that go on the stack: an intervening "if" is checked as it triggers (and again as it resolves), and
/// a trigger limit each turn
fn waiting(game: &mut Game, mut pending: Vec<StackItem>) -> Vec<StackItem> {
    pending.retain(|t| {
        let ability = t.ability.and_then(|(i, _)| game.item_def(t).abilities.get(i));
        game.condition_holds(ability.and_then(|a| a.intervening_if()), t.controller, t.card)
            && game.pelt_holds(t)
            && game.condition_holds(t.delayed.as_ref().and_then(|dl| dl.condition.as_ref()), t.controller, t.card)
    });
    pending.retain(|t| {
        let Some((i, zcc)) = t.ability else { return true };
        let Some(AbilityDef::TriggeredAbility { limit_per_turn: Some(n), .. }) = game.item_def(t).abilities.get(i)
        else {
            return true;
        };
        // `TriggeredAbilityImpl.checkTriggeredLimit`: counted per object, so the source changing zones starts afresh;
        // the object it was when it triggered (Bloombrute's lifelink gain as it dies in combat)
        let key = (t.card, zcc, i);
        let seen = game.triggered_this_turn.iter().filter(|x| **x == key).count() as u32;
        if seen >= *n {
            return false;
        }
        game.triggered_this_turn.push(key);
        true
    });
    pending
}

fn same_trigger(game: &Game, a: &StackItem, b: &StackItem) -> bool {
    // `getRule(sourceName)`: "{this}" reads as the source's name
    let name = game.stack_name(a);
    name == game.stack_name(b)
        && (!name.contains("{this}") || game.obj(a.card).name() == game.obj(b.card).name())
        && a.targets == b.targets
        // delayed ones: their fixed targets, as `FixedTarget.getTargets` sees them (none once it changed zones)
        && live_fixed(game, a) == live_fixed(game, b)
}

fn live_fixed(game: &Game, t: &StackItem) -> Vec<ObjId> {
    // what it sacrifices is a `FixedTargets` pointer too (the tokens a mobilize made)
    t.delayed
        .as_ref()
        // blitz's sacrifice is a `FixedTarget` too
        .map(|dl| {
            dl.fixed
                .iter()
                .chain(dl.sacrifice.iter())
                .filter(|(o, z)| game.obj(*o).zcc == *z)
                .map(|(o, _)| *o)
                .collect()
        })
        .unwrap_or_default()
}

/// `TokenImpl.generateTokenInfo`: creating tokens (or an emblem) picks an image with the game's random when there are
/// two or more to pick from.
fn token_image(game: &mut Game, token: &crate::card::TokenDef, source: ObjId) {
    // a token's set is the one its image came from (`token.setExpansionSetCode(tokenInfo.getSetCode())`): a
    // Treasure from Fable's Goblin Shaman looks in NEO's Treasures, one image, so no random number
    let set = match &game.obj(source).token_image {
        Some((_, set, _)) if game.obj(source).token => set.clone(),
        _ => game.obj(source).def.set.clone(),
    };
    let n = token.image_choices(&set);
    let pick = if n >= 2 { game.rng.next_int_bound(n as i32) as u32 } else { 0 };
    game.token_made = Some((token.name.clone(), token.class.clone(), set, pick));
}

/// Damage from `source`: to a player it's life lost, to a creature it's marked. Lifelink gains its controller life.
/// Returns combat lifelink's life, which `combat_damage` gains once per source; outside combat, the damage dealt.
fn deal_damage(game: &mut Game, source: ObjId, to: Target, amount: i32, combat: bool) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let you = game.obj(source).controller;
    let permanent = game.obj(source).zone == Zone::Battlefield || game.lookback.contains(&source);
    // Valley Flamecaller: a Lizard, Mouse, Otter or Raccoon of yours deals that much damage plus 1
    let mut amount = amount;
    let mut flamecallers = 0;
    if permanent && ["Lizard", "Mouse", "Otter", "Raccoon"].iter().any(|t| game.obj(source).def.has_subtype(t)) {
        let n = game
            .battlefield
            .iter()
            .filter(|b| {
                game.obj(**b).controller == you
                    && game.obj(**b).def.abilities.iter().any(|a| match a {
                        AbilityDef::SimpleStaticAbility { effects, .. } => {
                            effects.iter().any(|e| matches!(e, EffectDef::ValleyFlamecallerEffect))
                        }
                        _ => false,
                    })
            })
            .count();
        amount += n as i32;
        flamecallers = n;
    }
    // a shield counter: removed, and the damage prevented (unless damage can't be prevented this turn, or it's combat
    // damage of yours that can't be)
    if let Target::Obj(o) = to {
        if game.obj(o).zone == Zone::Battlefield && game.obj(o).counter("shield") > 0 {
            game.obj_mut(o).add_counters("shield", -1);
            let unpreventable = game.no_prevention_turn == Some(game.turn)
                || combat
                    && game.any_static(|g, src, e| {
                        matches!(e, EffectDef::DamageCantBePreventedEffect { .. }) && g.obj(src).controller == you
                    });
            if !unpreventable {
                return 0;
            }
        }
    }
    // "prevent all combat damage a source of your choice would deal this turn" (Ajani's Aid)
    if combat && game.no_prevention_turn != Some(game.turn) {
        let (zcc, turn) = (game.obj(source).zcc, game.turn);
        if game.prevent_combat_from.contains(&(source, zcc, turn)) {
            return 0;
        }
    }
    // "prevent all damage that would be dealt to it this turn" (Gideon, Martial Paragon)
    if let Target::Obj(o) = to {
        let (zcc, turn) = (game.obj(o).zcc, game.turn);
        if game.prevent_all_to.contains(&(o, zcc, turn)) && game.no_prevention_turn != Some(turn) {
            return 0;
        }
    }
    // protection from the source's color: prevented (`checkProtectionAbilities`)
    if let Target::Obj(o) = to {
        if game.obj(o).zone == Zone::Battlefield && game.protected_from(o, source) {
            let unpreventable = game.no_prevention_turn == Some(game.turn)
                || combat
                    && game.any_static(|g, src, e| {
                        matches!(e, EffectDef::DamageCantBePreventedEffect { .. }) && g.obj(src).controller == you
                    });
            if !unpreventable {
                return 0;
            }
        }
    }
    // Artist's Talent: noncombat damage from your source to an opponent or their permanent, plus 2
    let mut talents: usize = 0;
    if !combat {
        let opponent = match to {
            Target::Player(p) => p != you,
            Target::Obj(o) => game.obj(o).controller != you,
        };
        if opponent {
            let mut n = 0;
            game.any_static(|g, src, e| {
                if let EffectDef::ArtistsTalentEffect { condition } = e {
                    if g.obj(src).controller == you && g.condition_holds(condition.as_ref(), you, src) {
                        n += 2;
                    }
                }
                false
            });
            amount += n;
            talents = (n / 2) as usize;
        }
    }
    // Twinflame Tyrant: damage from your source to an opponent or a permanent of theirs is doubled, once per Tyrant
    let opponent = match to {
        Target::Player(p) => p != you,
        Target::Obj(o) => game.obj(o).controller != you,
    };
    if opponent {
        let mut tyrants = 0;
        game.any_static(|g, src, e| {
            if matches!(e, EffectDef::TwinflameTyrantEffect) && g.obj(src).controller == you {
                tyrants += 1;
            }
            false
        });
        for _ in 0..tyrants {
            amount = amount.saturating_mul(2);
        }
        talents += tyrants;
    }
    // two or more of these (Flamecallers, Talents, Tyrants): the damaged player (or the permanent's controller)
    // orders them (`chooseReplacementEffect`; two of one card differ by their object's name and id)
    let chooser = match to {
        Target::Player(p) => p,
        Target::Obj(o) => game.obj(o).controller,
    };
    for _ in 1..flamecallers + talents {
        game.owed_choices.push(chooser);
    }
    // Soul-Scar Mage: noncombat damage from your source to an opponent's creature is -1/-1 counters instead
    if let Target::Obj(o) = to {
        let mages = game
            .battlefield
            .iter()
            .filter(|b| {
                game.obj(**b).controller == you
                    && game.obj(**b).def.abilities.iter().any(|a| match a {
                        AbilityDef::SimpleStaticAbility { effects, .. } => {
                            effects.iter().any(|e| matches!(e, EffectDef::SoulScarMageDamageReplacementEffect))
                        }
                        _ => false,
                    })
            })
            .count();
        if !combat
            && game.obj(o).zone == Zone::Battlefield
            && game.has_type(o, CardType::Creature)
            && game.obj(o).controller != you
            && mages > 0
        {
            // two or more: the creature's controller picks which applies (they all do the same)
            if mages > 1 {
                let c = game.obj(o).controller;
                game.owed_choices.push(c);
            }
            game.obj_mut(o).add_counters("-1/-1", amount);
            return if combat { 0 } else { amount };
        }
    }
    let mut lifelink = 0;
    let src = game.obj(source).clone();
    // keywords it has now (granted ones too), or had as it left
    let keyword = |a: &AbilityDef| if src.zone == Zone::Battlefield { game.has_ability(source, a) } else { src.has(a) };
    let (src_deathtouch, src_lifelink) =
        (keyword(&AbilityDef::DeathtouchAbility), keyword(&AbilityDef::LifelinkAbility));
    match to {
        Target::Player(s) => {
            game.record(GameEvent {
                target: Some(to),
                source: Some(source),
                amount,
                combat,
                ..GameEvent::new(EventKind::Damage)
            });
            lose_life_by(game, s, amount, Some(source), combat);
            // toxic: combat damage to a player also gives them poison counters, each toxic ability's (`doDamage`)
            if combat {
                let toxic = |a: &AbilityDef| match a {
                    AbilityDef::ToxicAbility { amount, .. } => *amount,
                    _ => 0,
                };
                let mut n: i32 = src.def.abilities.iter().map(toxic).sum();
                if src.zone == Zone::Battlefield {
                    n += game.granted_abilities(source).iter().map(toxic).sum::<i32>();
                }
                add_poison(game, s, n);
            }
            if !combat {
                damaged_opponent_triggers(game, s);
            }
        }
        Target::Obj(o) => {
            if game.obj(o).zone != Zone::Battlefield {
                return 0;
            }
            let deathtouch = src_deathtouch;
            let creature = game.has_type(o, CardType::Creature);
            if game.obj(o).def.abilities.iter().any(|a| a.trigger() == Some(&Trigger::DealtDamage)) {
                game.damaged_batch.push((o, amount));
            }
            let planeswalker = game.has_type(o, CardType::Planeswalker);
            let battle = game.has_type(o, CardType::Battle);
            if log::kept(game) {
                // `PermanentImpl.doDamage`
                let line = format!("{} deals {} damage to {}", log::object(game, source), amount, log::object(game, o));
                log::inform(game, line);
            }
            game.record(GameEvent {
                target: Some(to),
                source: Some(source),
                amount,
                combat,
                ..GameEvent::new(EventKind::Damage)
            });
            if creature {
                let zcc = game.obj(o).zcc;
                game.damaged_by_source.push((source, o, zcc));
            }
            let turn = game.turn;
            let t = game.obj_mut(o);
            t.damaged_by = if t.damaged_by.0 == turn { (turn, t.damaged_by.1 | (1 << you)) } else { (turn, 1 << you) };
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
            // a battle, defense counters; the last one removed triggers a Siege
            if battle {
                let n = amount.min(t.counter("defense"));
                t.add_counters("defense", -n);
                if n > 0 && t.counter("defense") == 0 {
                    let fired = game.triggers([o], |_, _, _, t| matches!(t, Trigger::SiegeDefeated));
                    game.pending.extend(fired);
                }
            }
        }
    }
    if src_lifelink {
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
            // the damage dealt (`SavedDamageValue.MUCH`)
            t.x = amount;
            game.pending.push(t);
        }
    }
    if combat
        && (matches!(to, Target::Player(_)) || matches!(to, Target::Obj(o) if game.has_type(o, CardType::Planeswalker)))
    {
        let fired = game.triggers([source], |_, _, _, t| matches!(t, Trigger::CombatDamageToPlayerOrPlaneswalker));
        game.pending.extend(fired);
    }
    if combat {
        if let Target::Player(p) = to {
            let fired = game.triggers([source], |_, _, _, t| matches!(t, Trigger::CombatDamageToPlayer));
            let mut own = Vec::new();
            for mut t in fired {
                let a = t.ability.and_then(|(i, _)| game.item_def(&t).abilities.get(i));
                if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. })) {
                    t.targets = vec![vec![Target::Player(p)]];
                    t.target_zcc = vec![vec![0]];
                }
                // the damage dealt (`SavedDamageValue.MANY`)
                t.x = amount;
                own.push(t);
            }
            let sources = game.trigger_sources();
            let fired = game.triggers(sources, |g, src, you, t| {
                matches!(t, Trigger::CombatDamageToPlayerAll { filter } if g.filter_matches(filter, you, Some(src), source))
            });
            // one damage event: Java checks every trigger in its load order; "that many" is the damage (Starwinder)
            own.extend(fired.into_iter().map(|mut t| {
                t.x = amount;
                t
            }));
            own.sort_by_key(|t| game.trigger_key(t));
            game.pending.extend(own);
        }
    }
    if combat {
        lifelink
    } else {
        amount
    }
}

/// Noncombat damage to `seat`: "whenever one or more opponents are dealt noncombat damage" triggers, once per batch
/// (a resolution's damage is one batch: a trigger already waiting isn't added again).
fn damaged_opponent_triggers(game: &mut Game, seat: Seat) {
    let sources: Vec<ObjId> = game.trigger_sources();
    let batch = game.mana_batch;
    // a mana ability's damage (a painland's) is a batch of its own: one waiting from an earlier one still adds
    let fired = game.triggers(sources, |g, src, you, t| {
        matches!(t, Trigger::OpponentsDealtNoncombatDamage)
            && you != seat
            && !(g.pending.iter().any(|p| p.card == src && same_ability(g, p, t))
                && g.noncombat_fired.contains(&(src, batch)))
    });
    game.noncombat_fired.retain(|(_, b)| *b == batch);
    game.noncombat_fired.extend(fired.iter().map(|t| (t.card, batch)));
    game.pending.extend(fired);
}

fn same_ability(game: &Game, p: &StackItem, t: &Trigger) -> bool {
    p.ability.and_then(|(i, _)| game.item_def(p).abilities.get(i)).and_then(|a| a.trigger()) == Some(t)
}

/// `seat` gains life: "whenever you gain life" triggers (`GAINED_LIFE`).
/// Life lost (damage, paying life, "loses life"), counted for the turn (`PlayerLostLifeWatcher`).
pub fn lose_life(game: &mut Game, seat: Seat, amount: i32) {
    let source = log::source(game);
    lose_life_by(game, seat, amount, source, false);
}

/// Loses life to `source` (`PlayerImpl.loseLife`; `combat`: to combat damage)
fn lose_life_by(game: &mut Game, seat: Seat, amount: i32, source: Option<ObjId>, combat: bool) {
    if log::kept(game) {
        let mut line = format!("{} loses {} life", log::player(seat), amount);
        if combat {
            line += " at combat";
        }
        if let Some(s) = source {
            line += &format!(" from {}", log::object(game, s));
        }
        log::inform(game, line);
    }
    if amount > 0 {
        game.record(GameEvent {
            target: Some(Target::Player(seat)),
            player: Some(seat),
            amount,
            ..GameEvent::new(EventKind::LifeLoss)
        });
    }
    let p = game.player_mut(seat);
    p.life -= amount;
    if amount > 0 {
        p.life_lost += amount;
        // `LOST_LIFE`: the life lost is the trigger's "that much" (`SavedLifeLossValue`), read as its X
        let sources: Vec<ObjId> = game.trigger_sources();
        let fired = game.triggers(sources, |_, _, you, t| match t {
            Trigger::LosesLife { who } => Game::whose(who, you, seat),
            _ => false,
        });
        // a batch for each player: one trigger, with all the life they lost
        for t in fired {
            match game.life_batch.iter_mut().find(|(s, b)| *s == seat && b.card == t.card && b.ability == t.ability) {
                Some((_, b)) => b.x += amount,
                None => game.life_batch.push((seat, StackItem { x: amount, ..t })),
            }
        }
    }
}

pub fn gain_life(game: &mut Game, seat: Seat, amount: i32) {
    if amount <= 0
        || game.player(seat).cant_gain_life
        || game.no_lifegain_turn == Some(game.turn)
        || game.any_static(|_, _, e| matches!(e, EffectDef::CantGainLifeAllEffect))
    {
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
    // two or more (each one's text names its object, so they differ): the player picks which applies first, and
    // again while two are left (`HumanPlayer.chooseReplacementEffect`)
    for _ in 1..plus {
        game.owed_choices.push(seat);
    }
    let amount = amount + plus;
    if log::kept(game) {
        let mut line = format!("{} gains {} life", log::player(seat), amount);
        if let Some(s) = log::source(game) {
            line += &format!(" (source: {})", log::object(game, s));
        }
        log::inform(game, line);
    }
    game.player_mut(seat).life += amount;
    game.record(GameEvent {
        target: Some(Target::Player(seat)),
        player: Some(seat),
        amount,
        ..GameEvent::new(EventKind::LifeGain)
    });
    game.player_mut(seat).life_gains += 1;
    game.player_mut(seat).life_gained += amount;
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

pub fn can_attack(game: &Game, seat: Seat, id: ObjId) -> bool {
    let o = game.obj(id);
    o.controller == seat
        && o.is(CardType::Creature)
        && !o.tapped
        && o.attacking.is_none()
        && (!o.sick || game.has_ability(id, &AbilityDef::HasteAbility))
        && !game.has_ability(id, &AbilityDef::DefenderAbility)
        && !game.cant_attack_or_block(id)
        && !game.cant_attack_alone(id)
        && !game.has_ability(id, &AbilityDef::CantAttackAbility)
}

/// `Combat.selectAttackers` and `HumanPlayer.selectAttackers`: asked only when something can attack; "special" is
/// the All attack button, a creature toggles it, any other answer is done.
fn declare_attackers(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
    let defender = game.opponent(seat);
    let possible =
        |g: &Game| -> Vec<ObjId> { g.battlefield.iter().copied().filter(|id| can_attack(g, seat, *id)).collect() };
    // `checkAttackRequirements`, before the player is asked: what must attack does
    for id in possible(game) {
        if game.must_attack(id) {
            let Some(pw) = choose_defender(game, d, seat, defender) else { return };
            attack(game, id, defender, pw);
        }
    }
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
                } else if game.obj(id).attacking.is_some() && game.stack.is_empty() && !game.must_attack(id) {
                    let vigilance = game.has_ability(id, &AbilityDef::VigilanceAbility);
                    let o = game.obj_mut(id);
                    o.attacking = None;
                    o.attacking_pw = None;
                    if !vigilance {
                        o.tapped = false;
                    }
                    game.combat_groups.retain(|g| !(g.attacker == id && g.left.is_none()));
                }
            }
            _ => break,
        }
        if d.stopped() {
            return;
        }
    }
    // `checkAttackRestrictions`: one that can't attack alone, attacking alone, is removed
    loop {
        let attacking: Vec<ObjId> =
            game.battlefield.iter().copied().filter(|a| game.obj(*a).attacking.is_some()).collect();
        let Some(&alone) = attacking.iter().find(|a| {
            attacking.len() == 1 && has_static(game, **a, |e| matches!(e, EffectDef::CantAttackAloneSourceEffect))
        }) else {
            break;
        };
        let vigilance = game.has_ability(alone, &AbilityDef::VigilanceAbility);
        let o = game.obj_mut(alone);
        o.attacking = None;
        o.attacking_pw = None;
        if !vigilance {
            o.tapped = false;
        }
        game.combat_groups.retain(|g| g.attacker != alone);
    }
}

/// Whether a permanent's own static abilities have an effect
fn has_static(game: &Game, id: ObjId, f: impl Fn(&EffectDef) -> bool) -> bool {
    game.obj(id).def.abilities.iter().any(|a| match a {
        AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(&f),
        _ => false,
    })
}

/// `Combat.addAttackerToCombat`: a new group for the attacker
fn join_combat(game: &mut Game, id: ObjId) {
    game.combat_groups.push(crate::state::CombatGroup { attacker: id, left: None });
}

/// `HumanPlayer.selectDefender`: with the defending player the only defender there's no question; with their
/// planeswalkers too, PICK_TARGET (required: anything else asks again). Some(None): the player; None: stopped.
fn choose_defender(game: &mut Game, d: &mut dyn Decider, seat: Seat, defender: Seat) -> Option<Option<(ObjId, u32)>> {
    let mut pws: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| game.obj(*p).controller == defender && game.has_type(*p, CardType::Planeswalker))
        .collect();
    // battles an opponent of the attacker protects (`Combat.setDefenders`: after the players and planeswalkers)
    pws.extend(
        game.battlefield
            .iter()
            .copied()
            .filter(|b| game.has_type(*b, CardType::Battle) && game.obj(*b).protector == Some(defender)),
    );
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
    join_combat(game, id);
    let vigilance = game.has_ability(id, &AbilityDef::VigilanceAbility);
    let controller = game.obj(id).controller;
    note(game, EventKind::Attack, Some(controller), Some(id), None, 0);
    let o = game.obj_mut(id);
    o.attacking = Some(defender);
    o.attacking_pw = pw;
    if !vigilance {
        tap(game, id);
    }
}

/// Undoes a cost's tap (the cost wasn't paid): the "whenever this becomes tapped" triggers it fired go too
fn untap_undo(game: &mut Game, id: ObjId) {
    game.obj_mut(id).tapped = false;
    let def = game.obj(id).def.clone();
    game.pending.retain(|t| {
        !(t.card == id
            && t.delayed.is_none()
            && t.ability.is_some_and(|(i, _)| {
                matches!(def.abilities.get(i).and_then(|a| a.trigger()), Some(Trigger::BecomesTapped))
            }))
    });
}

/// Taps a permanent: "whenever this becomes tapped" triggers
fn tap(game: &mut Game, id: ObjId) {
    if game.obj(id).tapped {
        return;
    }
    game.obj_mut(id).tapped = true;
    let fired = game.triggers([id], |_, _, _, t| matches!(t, Trigger::BecomesTapped));
    game.pending.extend(fired);
}

pub fn can_block(game: &Game, blocker: ObjId, attacker: ObjId) -> bool {
    if game.has_ability(attacker, &AbilityDef::CantBeBlockedSourceAbility)
        || game.cant_attack_or_block(blocker)
        || game.has_ability(blocker, &AbilityDef::CantBlockAbility)
        || (game.has_ability(attacker, &AbilityDef::DauntAbility) && game.power_toughness(blocker).0 <= 2)
        || (game.has_ability(attacker, &AbilityDef::IslandwalkAbility) && {
            let defender = game.obj(blocker).controller;
            game.battlefield.iter().any(|l| game.obj(*l).controller == defender && game.has_subtype(*l, "Island"))
        })
        || game.protected_from(attacker, blocker)
    {
        return false;
    }
    // Gingerbrute: creatures of a filter can't block it this turn
    let a = game.obj(attacker);
    if game.grants.iter().any(|g| {
        g.target == attacker
            && g.zcc == a.zcc
            && matches!(&g.ability, AbilityDef::CantBeBlockedByAbility { filter }
                if game.filter_matches(filter, a.controller, Some(attacker), blocker))
    }) {
        return false;
    }
    // Stromkirk Noble: its own "can't be blocked by Humans" (`CantBeBlockedByCreaturesSourceEffect`)
    if game.obj(attacker).def.abilities.iter().any(|x| match x {
        AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| {
            matches!(e, EffectDef::CantBeBlockedByCreaturesSourceEffect { filter, duration }
                if *duration == Duration::WhileOnBattlefield
                    && game.filter_matches(filter, a.controller, Some(attacker), blocker))
        }),
        _ => false,
    }) {
        return false;
    }
    // Elusive Otter: creatures with power less than its can't block it
    if game.obj(attacker).def.abilities.iter().any(|a| match a {
        AbilityDef::SimpleStaticAbility { effects, .. } => {
            effects.iter().any(|e| matches!(e, EffectDef::CantBeBlockedByCreaturesWithLessPowerEffect))
        }
        _ => false,
    }) && game.power_toughness(blocker).0 < game.power_toughness(attacker).0
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
    let mut tries = 0;
    loop {
        tries += 1;
        select_blockers(game, d, seat);
        if d.stopped() {
            return;
        }
        let mut legal = true;
        for a in attackers(game) {
            // "can't be blocked by more than one creature" (an Aura's static): every blocker is removed
            if max_blocked_by_one(game, a) && live_blockers(game, a).len() > 1 {
                for b in live_blockers(game, a) {
                    game.obj_mut(b).blocking.clear();
                    game.obj_mut(b).blocks = 0;
                }
                let o = game.obj_mut(a);
                o.blocked_by.clear();
                o.blocked = false;
                legal = false;
                continue;
            }
            if !game.has_ability(a, &AbilityDef::MenaceAbility) {
                continue;
            }
            let blockers = live_blockers(game, a);
            if blockers.len() != 1 {
                continue;
            }
            game.obj_mut(blockers[0]).blocking.clear();
            game.obj_mut(blockers[0]).blocks = 0;
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
        // `checkBlockRequirementsAfter`: a creature that blocks if able and isn't blocking, though it could block an
        // attacker (not one that needs more blockers): asked again, the blocks as they are
        // (a player that never makes the block gets its blocks after many tries, rather than asked forever)
        // "must be blocked if able": unblocked though one of the player's creatures could block it, asked again
        if legal && tries < 100 {
            for a in attackers(game) {
                if has_static(game, a, |e| matches!(e, EffectDef::MustBeBlockedByAtLeastOneSourceEffect))
                    && live_blockers(game, a).is_empty()
                    && game.battlefield.iter().any(|b| {
                        let o = game.obj(*b);
                        o.controller == seat && o.is(CardType::Creature) && !o.tapped && can_block(game, *b, a)
                    })
                {
                    legal = false;
                }
            }
        }
        if legal && tries < 100 {
            let attacking = attackers(game);
            legal = !game.battlefield.iter().any(|b| {
                let o = game.obj(*b);
                o.controller == seat
                    && !o.tapped
                    && o.blocking.is_empty()
                    && game.has_ability(*b, &AbilityDef::BlocksIfAbleAbility)
                    && attacking.iter().any(|a| {
                        can_block(game, *b, *a)
                            && !game.has_ability(*a, &AbilityDef::MenaceAbility)
                            && (!max_blocked_by_one(game, *a) || live_blockers(game, *a).is_empty())
                    })
            });
        }
        // `checkBlockRestrictionsAfter`: one that can't block alone, blocking alone, is removed
        let blocking: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|b| game.obj(*b).controller == seat && !game.obj(*b).blocking.is_empty())
            .collect();
        if blocking.len() == 1 && has_static(game, blocking[0], |e| matches!(e, EffectDef::CantBlockAloneSourceEffect))
        {
            let b = blocking[0];
            for a in std::mem::take(&mut game.obj_mut(b).blocking) {
                game.obj_mut(a).blocked_by.retain(|x| *x != b);
            }
            game.obj_mut(b).blocks = 0;
            refresh_blocked(game);
        }
        if legal {
            return;
        }
    }
}

/// Whether an Aura on `a` says it can't be blocked by more than one creature.
fn max_blocked_by_one(game: &Game, a: ObjId) -> bool {
    game.any_static(|g, src, e| {
        matches!(e, EffectDef::CantBeBlockedByMoreThanOneAttachedEffect) && g.attached(src) == Some(a)
    })
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
                // `Combat.removeBlocker`: out of each group it blocked, too (the view's blockers)
                for a in std::mem::take(&mut game.obj_mut(blocker).blocking) {
                    game.obj_mut(a).blocked_by.retain(|x| *x != blocker);
                }
                game.obj_mut(blocker).blocks = 0;
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
            game.obj_mut(blocker).blocks += 1;
            let controller = game.obj(blocker).controller;
            note(game, EventKind::Block, Some(controller), Some(blocker), Some(Target::Obj(a)), 0);
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

/// `CombatGroup.dealsDamageThisStep`
fn deals_damage_in(game: &mut Game, id: ObjId, first: bool) -> bool {
    let fs = game.has_ability(id, &AbilityDef::FirstStrikeAbility);
    let ds = game.has_ability(id, &AbilityDef::DoubleStrikeAbility);
    if first {
        if fs || ds {
            game.first_strikers.push(id);
        }
        fs || ds
    } else {
        ds || !game.first_strikers.contains(&id)
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

/// Damage that's lethal to `blocker` from `attacker` (`Permanent.getLethalDamage`: the attacker's current abilities,
/// granted deathtouch too).
fn lethal_damage(game: &Game, blocker: ObjId, attacker: ObjId) -> i32 {
    let (_, t) = game.power_toughness(blocker);
    let lethal = (t - game.obj(blocker).damage).max(0);
    if game.has_ability(attacker, &AbilityDef::DeathtouchAbility) {
        lethal.min(1)
    } else {
        lethal
    }
}

/// `ProliferateEffect`: one `HumanPlayer.choose` of a `TargetPermanentOrPlayer(0, MAX)` of what has counters (not
/// targeted), asked even with nothing to choose; then, in the order chosen, one more counter of each kind there (a
/// permanent's in `Counters`' hash order). False: the decider stopped.
fn proliferate(game: &mut Game, d: &mut dyn Decider, seat: Seat) -> bool {
    let has = |c: &[(String, i32)]| c.iter().any(|(_, k)| *k > 0);
    let objs: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| has(&game.obj(*o).counters)).collect();
    let players: Vec<Seat> =
        (0..game.seats()).filter(|s| !game.player(*s).lost && has(&game.player(*s).counters)).collect();
    let mut chosen: Vec<Target> = Vec::new();
    loop {
        let left_objs: Vec<ObjId> = objs.iter().copied().filter(|o| !chosen.contains(&Target::Obj(*o))).collect();
        let left_players: Vec<Seat> =
            players.iter().copied().filter(|p| !chosen.contains(&Target::Player(*p))).collect();
        let q = Query::Target { players: left_players.clone(), objs: left_objs.clone(), required: false };
        let pick = match ask(game, d, seat, &q) {
            Answer::Obj(o) => Target::Obj(o),
            Answer::Player(p) => Target::Player(p),
            Answer::Stop => return false,
            Answer::Unknown => continue,
            _ => break,
        };
        if chosen.contains(&pick) {
            chosen.retain(|t| *t != pick);
            continue;
        }
        let possible = match pick {
            Target::Obj(o) => left_objs.contains(&o),
            Target::Player(p) => left_players.contains(&p),
        };
        if possible {
            chosen.push(pick);
            // `TargetImpl.isChoiceCompleted`: nothing more to choose
            if chosen.len() == objs.len() + players.len() {
                break;
            }
        }
        if d.stopped() {
            return false;
        }
    }
    for t in chosen {
        match t {
            Target::Obj(o) => {
                if game.obj(o).zone != Zone::Battlefield {
                    continue;
                }
                let kinds: Vec<String> =
                    game.obj(o).counters.iter().filter(|(_, k)| *k > 0).map(|(n, _)| n.clone()).collect();
                for name in java_hash_order(kinds) {
                    put_counters(game, o, &name, 1, seat);
                }
            }
            Target::Player(p) => {
                let kinds: Vec<String> =
                    game.player(p).counters.iter().filter(|(_, k)| *k > 0).map(|(n, _)| n.clone()).collect();
                for name in java_hash_order(kinds) {
                    if name == "poison" {
                        add_poison(game, p, 1);
                    } else if let Some(c) = game.player_mut(p).counters.iter_mut().find(|(c, _)| *c == name) {
                        c.1 += 1;
                    }
                }
            }
        }
    }
    true
}

/// The order a Java `HashMap<String, _>` (`Counters`) lists keys inserted in this order: by bucket of the spread
/// `String.hashCode`, then insertion order within a bucket.
fn java_hash_order(keys: Vec<String>) -> Vec<String> {
    let mut cap = 16usize;
    while keys.len() * 4 > cap * 3 {
        cap *= 2;
    }
    let bucket = |k: &String| {
        let h = k.encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32));
        ((h ^ ((h as u32) >> 16) as i32) as u32 as usize) & (cap - 1)
    };
    let mut keyed: Vec<(usize, usize, String)> =
        keys.into_iter().enumerate().map(|(i, k)| (bucket(&k), i, k)).collect();
    keyed.sort();
    keyed.into_iter().map(|(_, _, k)| k).collect()
}

/// `HumanPlayer.getMultiAmountWithIndividualConstraints`: asked unless there's nothing to choose; only a string of
/// good values ends it.
fn multi_amount(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    defaults: Vec<i32>,
    ranges: Vec<(i32, i32)>,
    (min, max): (i32, i32),
    about: (Option<ObjId>, Vec<ObjId>),
) -> Option<Vec<i32>> {
    let n = defaults.len();
    if n == 0 || (n == 1 && min == max) || ranges.iter().all(|(lo, hi)| lo == hi) {
        return Some(defaults);
    }
    let (source, items) = about;
    let q = Query::MultiAmount { defaults, ranges: ranges.clone(), min, max, source, items };
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
    let mut gains: Vec<(ObjId, Seat, i32)> = Vec::new();
    // `CombatDamageStep`: each attacker's group deals its damage (and asks its replacement choices) before the next
    // group assigns its own
    // powers as the step starts: Java doesn't apply continuous effects again between groups, so a creature's
    // damage is what its power was before any was dealt (Righteous Valkyrie keeps its +2/+2 though the life it
    // needs is gone)
    let powers: Vec<(ObjId, i32)> = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).attacking.is_some() || !game.obj(**o).blocking.is_empty())
        .map(|o| (*o, game.power_toughness(*o).0))
        .collect();
    let power_of = |game: &Game, o: ObjId| match powers.iter().find(|(p, _)| *p == o) {
        Some((_, n)) => *n,
        None => game.power_toughness(o).0,
    };
    for a in attackers(game) {
        let mut hits: Vec<(ObjId, Target, i32)> = Vec::new();
        if !deals_damage_in(game, a, first) {
            continue;
        }
        let power = power_of(game, a);
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
            deal_hits(game, hits, &mut gains);
            settle_owed(game, d);
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
                match multi_amount(
                    game,
                    d,
                    seat,
                    defaults,
                    ranges,
                    (damage - remaining, damage),
                    (Some(a), blockers.clone()),
                ) {
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
                match multi_amount(game, d, seat, defaults, ranges, (damage, damage), (Some(a), blockers.clone())) {
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
        deal_hits(game, hits, &mut gains);
        settle_owed(game, d);
    }
    let mut hits: Vec<(ObjId, Target, i32)> = Vec::new();
    for b in Arc::clone(&game.battlefield).iter().copied() {
        let blocking = game.obj(b).blocking.clone();
        if blocking.is_empty() || !deals_damage_in(game, b, first) {
            continue;
        }
        let power = power_of(game, b);
        // not an attacker that left the battlefield (bounced and cast again, it's the same object here)
        if let Some(a) = blocking.first().filter(|a| game.obj(**a).blocked_by.contains(&b)) {
            hits.push((b, Target::Obj(*a), power));
        }
    }
    deal_hits(game, hits, &mut gains);
    // combat lifelink: each source's damage is gained at once, in the order the sources dealt it
    for (_, seat, amount) in gains {
        gain_life(game, seat, amount);
    }
}

/// Deals combat damage, adding up what each source dealt (for lifelink)
fn deal_hits(game: &mut Game, hits: Vec<(ObjId, Target, i32)>, gains: &mut Vec<(ObjId, Seat, i32)>) {
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
}

// ------------------------------------------------------------------------------------------------ cleanup and SBAs

/// Discard to hand size (one card per answer, as `HumanPlayer.choose` asks), then damage wears off.
fn cleanup(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
    let unlimited =
        game.any_static(|g, src, e| matches!(e, EffectDef::NoMaximumHandSizeEffect) && g.obj(src).controller == seat);
    let excess = if unlimited { 0 } else { game.player(seat).hand.len().saturating_sub(MAX_HAND) };
    if excess > 0 {
        if log::kept(game) {
            let line = format!("{} discards down to {} hand cards", log::player(seat), MAX_HAND);
            log::inform(game, line);
        }
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
            discard_card(game, seat, id);
        }
    }
    if game.boosts.iter().any(|b| b.until_end_of_turn) {
        Arc::make_mut(&mut game.boosts).retain(|b| !b.until_end_of_turn);
    }
    if game.grants.iter().any(|g| g.until_end_of_turn) {
        Arc::make_mut(&mut game.grants).retain(|g| !g.until_end_of_turn);
    }
    game.must_block_from.clear();
    game.exile_if_dies.clear();
    game.damaged_by_source.clear();
    if !game.all_basics_until_eot.is_empty() {
        game.all_basics_until_eot.clear();
        game.update_guildpact();
    }
    game.subtypes_until_eot.clear();
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
        if game.obj(id).regen_shields != 0 {
            game.obj_mut(id).regen_shields = 0;
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

/// Transforms a permanent into its other face; transforming into its front fires "enters or transforms into"
/// triggers (`PermanentImpl.transform`)
fn transform(game: &mut Game, id: ObjId) {
    // its abilities waiting or on the stack keep the face they came from
    let old = game.obj(id).def.clone();
    for t in Arc::make_mut(&mut game.stack).iter_mut().chain(game.pending.iter_mut()) {
        if t.card == id && t.ability.is_some() && t.def.is_none() {
            t.def = Some(old.clone());
        }
    }
    let o = game.obj_mut(id);
    let into_front = match o.front.take() {
        Some(front) => {
            o.def = front;
            true
        }
        None => {
            let Some(back) = o.def.transform_back.clone() else { return };
            o.front = Some(std::mem::replace(&mut o.def, back));
            o.transformed_in_place = true;
            false
        }
    };
    if into_front {
        let fired = game.triggers([id], |_, _, _, t| matches!(t, Trigger::TransformsOrEnters));
        game.pending.extend(fired);
    }
}

/// Day and night: with a daybound (nightbound) permanent and neither, it becomes day (night); then a daybound
/// permanent at night and a nightbound one by day transform (`GameImpl.checkStateBasedActions`)
fn day_night(game: &mut Game) {
    let bound = |g: &Game, a: &AbilityDef| g.battlefield.iter().any(|b| g.obj(*b).def.has(a));
    if game.day.is_none() {
        if bound(game, &AbilityDef::DayboundAbility) {
            game.day = Some(true);
        } else if bound(game, &AbilityDef::NightboundAbility) {
            game.day = Some(false);
        } else {
            return;
        }
    }
    let day = game.day == Some(true);
    for id in Arc::clone(&game.battlefield).iter().copied() {
        let def = &game.obj(id).def;
        if (def.has(&AbilityDef::DayboundAbility) && !day) || (def.has(&AbilityDef::NightboundAbility) && day) {
            transform(game, id);
        }
    }
}

/// True: one was performed.
pub fn state_based_actions(game: &mut Game) -> bool {
    refresh_becomes(game);
    day_night(game);
    let mut happened = false;
    loop {
        let mut any = false;
        // 704.5e: a copy of a card outside the stack and battlefield ceases to exist (one that may be cast stays)
        if game.exile.iter().any(|c| game.obj(*c).token && game.obj(*c).may_play.is_none()) {
            let gone: Vec<ObjId> =
                game.exile.iter().copied().filter(|c| game.obj(*c).token && game.obj(*c).may_play.is_none()).collect();
            Arc::make_mut(&mut game.exile).retain(|c| !gone.contains(c));
        }
        for s in 0..game.seats() {
            // the game is over once one player is left (`PlayerImpl.lostForced` concedes the loser, and
            // `checkIfGameIsOver` makes the one left the winner): when both are at 0 life the first in seat order
            // loses and the other wins, as in Java, though 704.5a and 104.4a would make it a draw
            if (0..game.seats()).filter(|t| !game.player(*t).lost).count() <= 1 {
                break;
            }
            let poisoned = poison(game, s) >= 10;
            let p = game.player(s);
            // 704.5a-c: 0 or less life, drawing from an empty library, ten or more poison counters
            if !p.lost && (p.life <= 0 || p.drew_from_empty || poisoned) {
                if log::kept(game) {
                    // `GameImpl.checkStateBasedActions`, `PlayerImpl.lost`: the last reason that holds is logged
                    let why = if poisoned {
                        "to having 10 or more poison counters"
                    } else if p.drew_from_empty {
                        "drawing from an empty library"
                    } else {
                        "to having 0 or less life"
                    };
                    let line = format!("{} has lost the game", log::player(s));
                    log::inform(game, line);
                    let line = format!("{} lost the game due {}", log::player(s), why);
                    log::inform(game, line);
                }
                let p = game.player_mut(s);
                p.lost = true;
                p.drew_from_empty = false;
                any = true;
            }
        }
        // 704.5s: a Saga with its last chapter reached, and none of its chapter abilities waiting or on the stack
        let chapter_of = |g: &Game, t: &StackItem, id: ObjId| {
            t.card == id
                && t.ability.is_some_and(|(i, _)| {
                    matches!(g.obj(id).def.abilities.get(i).and_then(|a| a.trigger()), Some(Trigger::Chapter { .. }))
                })
        };
        let done: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                let o = game.obj(*id);
                o.def
                    .abilities
                    .iter()
                    .any(|a| matches!(a, AbilityDef::SagaAbility { max, .. } if o.counter("lore") >= *max as i32))
                    && !game.stack.iter().any(|t| chapter_of(game, t, *id))
                    && !game.pending.iter().any(|t| chapter_of(game, t, *id))
            })
            .collect();
        for id in done {
            sacrifice(game, id);
            any = true;
        }
        // 704.5q: +1/+1 and -1/-1 counters on a permanent cancel out in pairs
        for id in game.battlefield.iter().copied().collect::<Vec<_>>() {
            let n = game.obj(id).counter("+1/+1").min(game.obj(id).counter("-1/-1"));
            if n > 0 {
                let o = game.obj_mut(id);
                o.add_counters("+1/+1", -n);
                o.add_counters("-1/-1", -n);
            }
        }
        let alive: Vec<Seat> = (0..game.seats()).filter(|s| !game.player(*s).lost).collect();
        if alive.len() <= 1 && !game.over && alive.len() < game.seats() {
            // nobody is marked as the winner yet; the fingerprint hides the loser's cards
            if let (Some(w), true) = (alive.first(), log::kept(game)) {
                let line = format!("{} has won the game", log::player(*w));
                log::inform(game, line);
            }
            game.over = true;
            return true;
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
                // 704.5v: a battle with no defense, unless an ability it triggered is waiting or on the stack
                if game.has_type(*id, CardType::Battle)
                    && o.counter("defense") == 0
                    && !game.stack.iter().chain(game.pending.iter()).any(|t| t.card == *id && t.ability.is_some())
                {
                    return true;
                }
                // 704.5m: an Aura attached to nothing, or to what it can't enchant
                if o.def.has_subtype("Aura") && !o.is(CardType::Creature) {
                    let Some(to) = game.attached(*id) else { return true };
                    if game.protected_from(to, *id) {
                        return true;
                    }
                    if let Some(TargetDef::TargetPermanent { filter, .. }) = o.def.spell.targets.first() {
                        if !game.filter_matches(filter, o.controller, Some(*id), to) {
                            return true;
                        }
                    }
                    // a Role token (no spell to carry its target) enchants a creature: one on a land whose
                    // animation ended goes (Restless Anchorage at cleanup)
                    if o.def.spell.targets.is_empty()
                        && o.def.has_subtype("Role")
                        && !game.has_type(to, CardType::Creature)
                    {
                        return true;
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
        // `GameImpl.checkStateBasedActions` moves each permanent as it comes to it: an Aura after the creature it
        // enchants, which went (for 0 toughness or destroyed by lethal damage), already enchants nothing (its card
        // follows the creature's)
        let bf: Vec<ObjId> = game.battlefield.to_vec();
        let gone_before = |aura: ObjId| {
            let Some(to) = game.attached(aura) else { return false };
            let (Some(a), Some(t)) = (bf.iter().position(|x| *x == aura), bf.iter().position(|x| *x == to)) else {
                return false;
            };
            t < a
                && dying.contains(&to)
                && game.has_type(to, CardType::Creature)
                && game.obj(aura).def.has_subtype("Aura")
        };
        // the Auras that follow their creature, only if it did leave (a regeneration shield keeps it)
        let follows: Vec<(ObjId, ObjId)> = bf
            .iter()
            .copied()
            .filter(|id| !dying.contains(id) && gone_before(*id))
            .filter_map(|id| game.attached(id).map(|to| (id, to)))
            .collect();
        let mut dying: Vec<ObjId> = bf.iter().copied().filter(|id| dying.contains(id) || gone_before(*id)).collect();
        // 704.5y: of the Roles one player controls on one permanent, only the newest stays
        let roles: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| game.obj(*id).def.has_subtype("Role") && !dying.contains(id))
            .collect();
        for r in &roles {
            let (o, c) = (game.obj(*r).attached_to, game.obj(*r).controller);
            if roles.iter().any(|x| *x > *r && game.obj(*x).attached_to == o && game.obj(*x).controller == c) {
                dying.push(*r);
            }
        }
        game.lookback = game.battlefield.to_vec();
        for id in dying {
            if follows.iter().any(|(a, to)| *a == id && game.obj(*to).zone == Zone::Battlefield) {
                continue;
            }
            // lethal damage destroys (a regeneration shield replaces that); 0 toughness doesn't
            let creature = game.has_type(id, CardType::Creature);
            let lethal = creature && game.power_toughness(id).1 > 0;
            if !(lethal && (shield(game, id) || umbra_armor(game, id) || regenerate(game, id))) {
                let token = game.obj(id).token;
                game.move_to(id, Zone::Graveyard);
                if log::kept(game) && creature && !token && game.obj(id).zone == Zone::Graveyard {
                    // `PermanentImpl.destroy` for lethal damage, `movePermanentToGraveyardWithInfo` for 0 toughness
                    let line = if lethal {
                        format!("{} died", log::object(game, id))
                    } else {
                        format!(
                            "{} is put into graveyard from battlefield (SBA: creature has toughness 0 or less)",
                            log::object(game, id)
                        )
                    };
                    log::inform(game, line);
                }
            }
            any = true;
        }
        game.lookback.clear();
        happened |= any;
        if !any {
            return happened;
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

    /// Pays with the pool: a mana type when asked to pay (`unlock`), else cancels; counts the questions
    struct PoolPayer {
        unlock: bool,
        asked: usize,
    }

    impl Decider for PoolPayer {
        fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
            match query {
                Query::PlayMana { .. } => {
                    self.asked += 1;
                    if self.unlock && self.asked == 1 {
                        Answer::ManaType("GREEN".into())
                    } else {
                        Answer::Bool(false)
                    }
                }
                _ => Passer.answer(game, seat, query),
            }
        }
    }

    #[test]
    fn the_pool_pays_as_the_players_payment_mode_says() {
        let db = crate::cards();
        let cost = ManaCost::parse("{G}").unwrap();
        // (manual, unrestricted, the mana was there before, unlocks) -> (paid, questions)
        for (manual, unrestricted, before, unlock, paid, asked) in [
            (false, false, false, false, true, 0),
            (false, false, true, false, false, 1),
            (false, true, true, false, true, 0),
            (true, false, false, false, false, 1),
            (true, false, false, true, true, 1),
        ] {
            let mut game = game_with(&db, &[("Forest", 20)]);
            game.player_mut(0).mana_pool = Mana::of(Color::G, 1);
            game.player_mut(0).mana_stock = if before { Mana::of(Color::G, 1) } else { Mana::default() };
            game.set_pool_mode(0, manual, unrestricted);
            let mut d = PoolPayer { unlock, asked: 0 };
            let ok = pay_mana_from(&mut game, &mut d, 0, &cost);
            assert_eq!((ok, d.asked), (paid, asked), "manual {manual}, unrestricted {unrestricted}, before {before}");
            assert_eq!(game.pool_mode(1), (false, false), "only the seat's own mode");
        }
    }

    /// Says yes to every yes/no question and keeps what it was asked.
    struct SaysYes(Vec<String>);

    impl Decider for SaysYes {
        fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
            match query {
                Query::YesNo { message } => {
                    self.0.push(format!("{seat}: {message}"));
                    Answer::Bool(true)
                }
                _ => Passer.answer(game, seat, query),
            }
        }
    }

    /// Logs every question and picks the first card offered
    struct CardLog(Vec<String>);

    impl Decider for CardLog {
        fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
            self.0.push(format!("{seat}: {query:?}"));
            match query {
                Query::Cards { objs, .. } => objs.first().map(|o| Answer::Obj(*o)).unwrap_or(Answer::Stop),
                _ => Passer.answer(game, seat, query),
            }
        }
    }

    #[test]
    fn you_win_the_game_ends_it_with_each_opponent_lost() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let src = game.create_obj(db.get("Forest").unwrap(), 0, Zone::Hand);
        game.move_to(src, Zone::Battlefield);
        let zcc = game.obj(src).zcc;
        let item = StackItem::spell(src, 0);
        apply_effects(&mut game, &mut Passer, &item, &[EffectDef::WinGameSourceControllerEffect], &[], (src, zcc));
        assert!(game.over);
        assert!(game.player(0).won && !game.player(0).lost);
        assert!(game.player(1).lost);
    }

    #[test]
    fn a_permanent_skips_only_its_controllers_next_untap() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let land = game.create_obj(db.get("Forest").unwrap(), 0, Zone::Hand);
        game.move_to(land, Zone::Battlefield);
        game.obj_mut(land).tapped = true;
        let item = StackItem::spell(land, 1);
        let zcc = game.obj(land).zcc;
        let targets = vec![vec![Target::Obj(land)]];
        apply_effects(
            &mut game,
            &mut Passer,
            &item,
            &[EffectDef::DontUntapInControllersNextUntapStepTargetEffect],
            &targets,
            (land, zcc),
        );
        assert_eq!(game.skip_next_untap, vec![(land, zcc)]);
    }

    #[test]
    fn kavus_discard_mode_asks_which_card() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let kavu = game.create_obj(db.get("Territorial Kavu").unwrap(), 0, Zone::Hand);
        game.move_to(kavu, Zone::Battlefield);
        let hand: Vec<ObjId> = (0..3).map(|_| game.create_obj(db.get("Forest").unwrap(), 0, Zone::Hand)).collect();
        game.player_mut(0).hand.extend(hand.iter().copied());
        let k = game.obj(kavu).def.abilities.iter().position(|a| a.trigger_modes().len() == 2).unwrap();
        let zcc = game.obj(kavu).zcc;
        Arc::make_mut(&mut game.stack).push(StackItem {
            ability: Some((k, zcc)),
            modes: vec![0],
            ..StackItem::spell(kavu, 0)
        });
        let mut d = CardLog(Vec::new());
        resolve_top(&mut game, &mut d);
        assert!(d.0.iter().any(|q| q.contains("Cards")), "the player picks the card to discard: {:?}", d.0);
    }

    #[test]
    fn a_fetchland_under_leyline_names_its_any_color_ability() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let leyline = game.create_obj(db.get("Leyline of the Guildpact").unwrap(), 0, Zone::Hand);
        game.move_to(leyline, Zone::Battlefield);
        let strand = game.create_obj(db.get("Flooded Strand").unwrap(), 0, Zone::Hand);
        game.move_to(strand, Zone::Battlefield);
        game.update_guildpact();
        let rules = crate::view::ability_rules(&game, strand).unwrap();
        assert_eq!(rules.len(), game.obj(strand).def.abilities.len());
        assert!(
            rules.iter().any(|r| r.contains("Sacrifice Flooded Strand") || r.contains("Sacrifice {this}")),
            "{rules:?}"
        );
        assert_eq!(rules.last().unwrap(), "{T}: Add one mana of any color.");
        let all: Vec<usize> = (0..rules.len()).collect();
        let p = crate::prompt::prompt_view(&game, 0, &Query::ChooseAbility { source: strand, abilities: all }).unwrap();
        for (_, text) in p["data"]["choices"].as_object().unwrap() {
            assert!(text.as_str().unwrap().len() > 4, "every choice has its rule: {text}");
        }
    }

    #[test]
    fn a_leyline_in_the_opening_hand_is_offered_at_the_start() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let leyline = game.create_obj(db.get("Leyline of the Guildpact").unwrap(), 1, Zone::Hand);
        game.player_mut(1).hand.push(leyline);
        let mut d = SaysYes(Vec::new());
        assert!(opening_hand_actions(&mut game, &mut d, 0));
        assert_eq!(d.0, vec!["1: Put Leyline of the Guildpact on the battlefield?".to_string()]);
        assert_eq!(game.obj(leyline).zone, Zone::Battlefield);
        assert!(!game.player(1).hand.contains(&leyline));
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

    fn on_battlefield(game: &mut Game, db: &crate::card::CardDb, name: &str, seat: Seat) -> ObjId {
        let id = game.create_obj(db.get(name).unwrap(), seat, Zone::Hand);
        game.move_to(id, Zone::Battlefield);
        game.obj_mut(id).sick = false;
        id
    }

    #[test]
    fn granted_deathtouch_makes_one_damage_lethal() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let bear = on_battlefield(&mut game, &db, "Bear Cub", 0);
        let sentry = on_battlefield(&mut game, &db, "Magnigoth Sentry", 1);
        assert_eq!(lethal_damage(&game, sentry, bear), game.power_toughness(sentry).1);
        grant(&mut game, bear, &AbilityDef::DeathtouchAbility);
        assert_eq!(lethal_damage(&game, sentry, bear), 1);
    }

    #[test]
    fn granted_defender_stops_an_attack() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let bear = on_battlefield(&mut game, &db, "Bear Cub", 0);
        let wall = on_battlefield(&mut game, &db, "Wall of Omens", 0);
        assert!(can_attack(&game, 0, bear));
        assert!(!can_attack(&game, 0, wall));
        grant(&mut game, bear, &AbilityDef::DefenderAbility);
        assert!(!can_attack(&game, 0, bear));
    }

    #[test]
    fn ten_poison_counters_lose_the_game_and_show_in_the_view() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        add_poison(&mut game, 1, 9);
        state_based_actions(&mut game);
        assert!(!game.player(1).lost && !game.over);
        add_poison(&mut game, 1, 1);
        assert_eq!(poison(&game, 1), 10);
        let view = crate::view::game_view(&game, 0, &["a", "b"]);
        let counters = &view["players"][1]["counters"];
        assert_eq!(counters, &serde_json::json!([{"name": "poison", "count": 10}]));
        state_based_actions(&mut game);
        assert!(game.player(1).lost && game.over);
        assert!(!game.player(0).lost);
    }

    /// Passes, and keeps the step of every priority it was asked at.
    struct PriorityLog(Vec<(Step, Seat)>);

    impl Decider for PriorityLog {
        fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
            if matches!(query, Query::Priority { .. }) {
                self.0.push((game.step.unwrap(), seat));
            }
            Passer.answer(game, seat, query)
        }
    }

    #[test]
    fn a_creature_dying_in_cleanup_gives_priority_and_another_cleanup() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        for s in 0..2 {
            game.player_mut(s).every_priority = true;
        }
        let bear = on_battlefield(&mut game, &db, "Bear Cub", 0);
        game.obj_mut(bear).add_counters("-1/-1", 2);
        let zcc = game.obj(bear).zcc;
        Arc::make_mut(&mut game.boosts).push(Boost {
            target: bear,
            zcc,
            power: 0,
            toughness: 2,
            until_end_of_turn: true,
        });
        game.step = Some(Step::EndTurn);
        let mut d = PriorityLog(Vec::new());
        play_from_priority(&mut game, &mut d, 2);
        assert_eq!(game.obj(bear).zone, Zone::Graveyard);
        // 514.3a: priority for both in the cleanup step the boost ended in; the next cleanup gives none
        let cleanup: Vec<Seat> = d.0.iter().filter(|(st, _)| *st == Step::Cleanup).map(|(_, s)| *s).collect();
        assert_eq!(cleanup, vec![0, 1]);
    }

    #[test]
    fn a_quiet_cleanup_gives_no_priority() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        for s in 0..2 {
            game.player_mut(s).every_priority = true;
        }
        on_battlefield(&mut game, &db, "Bear Cub", 0);
        game.step = Some(Step::EndTurn);
        let mut d = PriorityLog(Vec::new());
        play_from_priority(&mut game, &mut d, 2);
        assert!(d.0.iter().all(|(st, _)| *st != Step::Cleanup));
    }

    #[test]
    fn a_loop_of_one_source_is_offered_as_a_draw() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let bear = on_battlefield(&mut game, &db, "Bear Cub", 1);
        let mut d = SaysYes(Vec::new());
        for _ in 0..16 {
            check_infinite_loop(&mut game, &mut d, bear);
        }
        assert!(d.0.is_empty() && !game.over);
        check_infinite_loop(&mut game, &mut d, bear);
        // the source's controller is asked first
        let asked = "Draw game because of infinite looping?";
        assert_eq!(d.0, vec![format!("1: {asked}"), format!("0: {asked}")]);
        assert!(game.over && game.player(0).drew && game.player(1).drew);
        assert!(!game.player(0).lost && !game.player(1).lost);
    }

    #[test]
    fn a_declined_loop_counts_again_and_ends_in_the_end() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let bear = on_battlefield(&mut game, &db, "Bear Cub", 0);
        let mut n = 0;
        while !game.over {
            check_infinite_loop(&mut game, &mut Passer, bear);
            n += 1;
            assert!(n < 1000);
        }
        // the first detection after one resolution to remember the source and 16 to count, then 16 more each time
        assert_eq!(n, 1 + 16 * (LOOP_DECLINES as usize + 1));
        // a player losing life isn't looping
        let mut game = game_with(&db, &[("Forest", 10)]);
        for k in 0..40 {
            check_infinite_loop(&mut game, &mut Passer, bear);
            game.player_mut(k % 2).life -= 1;
        }
        assert!(!game.over && game.loop_declined == 0);
    }

    /// Picks the first card it may, and keeps what the search showed.
    struct Searcher(Vec<(usize, usize)>);

    impl Decider for Searcher {
        fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
            if let (Query::Cards { objs, .. }, Some((shown, chosen))) = (query, &game.searching) {
                self.0.push((shown.len(), chosen.len()));
                if let Some(o) = objs.first() {
                    return Answer::Obj(*o);
                }
            }
            Passer.answer(game, seat, query)
        }
    }

    #[test]
    fn a_library_search_shows_the_whole_library_sorted() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 5), ("Bear Cub", 5)]);
        let source = on_battlefield(&mut game, &db, "Bear Cub", 0);
        let filter = crate::card::Filter(vec![crate::card::Pred::Type { t: CardType::Creature }]);
        let library = game.player(0).library.len();
        let mut d = Searcher(Vec::new());
        let found = search_library(&mut game, &mut d, 0, &filter, 0, 2, source).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|c| game.obj(*c).name() == "Bear Cub"));
        assert_eq!(d.0, vec![(library, 0), (library, 1)]);
        assert!(game.searching.is_none());
    }

    #[test]
    fn subtypes_from_effects_count_in_the_rules() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Forest", 10)]);
        let squire = on_battlefield(&mut game, &db, "Skyknight Squire", 0);
        game.obj_mut(squire).add_counters("+1/+1", 2);
        assert!(!game.has_subtype(squire, "Knight"));
        game.obj_mut(squire).add_counters("+1/+1", 1);
        assert!(game.has_subtype(squire, "Knight"));
        let knights = crate::card::Filter(vec![crate::card::Pred::Subtype { s: "Knight".into() }]);
        assert!(game.filter_matches(&knights, 0, None, squire));
        // Sarkhan's "becomes a Dragon" until end of turn
        let sarkhan = on_battlefield(&mut game, &db, "Sarkhan, Dragon Ascendant", 0);
        let k = game.obj(sarkhan).def.abilities.iter().rposition(|a| a.trigger().is_some()).unwrap();
        let zcc = game.obj(sarkhan).zcc;
        Arc::make_mut(&mut game.stack).push(StackItem { ability: Some((k, zcc)), ..StackItem::spell(sarkhan, 0) });
        assert!(!game.has_subtype(sarkhan, "Dragon"));
        resolve_top(&mut game, &mut Passer);
        assert!(game.has_subtype(sarkhan, "Dragon"));
        assert!(game.has_ability(sarkhan, &AbilityDef::FlyingAbility));
        cleanup(&mut game, &mut Passer, 0);
        assert!(!game.has_subtype(sarkhan, "Dragon"));
    }

    /// Taps its mana sources first, then pays the rest with the spell's special action, picking every card or
    /// permanent offered; keeps the special actions it was offered.
    struct SpecialPayer {
        picked: Vec<ObjId>,
        offered: Vec<bool>,
    }

    impl Decider for SpecialPayer {
        fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
            match query {
                Query::PlayMana { sources, .. } => {
                    self.offered.push(game.special_mana);
                    let untapped = sources.iter().find(|s| !game.obj(**s).tapped);
                    match untapped {
                        Some(s) => Answer::Obj(*s),
                        None if game.special_mana => Answer::Text("special".into()),
                        None => Answer::Bool(false),
                    }
                }
                Query::ChooseAbility { source, abilities } => Answer::Ability(*source, abilities[0]),
                Query::Target { objs, .. } => match objs.iter().find(|o| !self.picked.contains(o)) {
                    Some(o) => {
                        self.picked.push(*o);
                        Answer::Obj(*o)
                    }
                    None => Answer::Bool(false),
                },
                _ => Passer.answer(game, seat, query),
            }
        }
    }

    #[test]
    fn delve_pays_with_the_graveyard_after_the_lands() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Island", 10)]);
        game.step = Some(Step::PrecombatMain);
        game.priority = Some(0);
        let island = on_battlefield(&mut game, &db, "Island", 0);
        for _ in 0..7 {
            let c = game.create_obj(db.get("Island").unwrap(), 0, Zone::Graveyard);
            game.player_mut(0).graveyard.push(c);
        }
        let cruise = game.create_obj(db.get("Treasure Cruise").unwrap(), 0, Zone::Hand);
        game.player_mut(0).hand.push(cruise);
        let mut d = SpecialPayer { picked: Vec::new(), offered: Vec::new() };
        assert!(cast(&mut game, &mut d, 0, cruise));
        assert_eq!(game.obj(cruise).zone, Zone::Stack);
        assert!(game.obj(island).tapped);
        assert!(game.player(0).graveyard.is_empty());
        assert_eq!(game.exile.len(), 7);
        assert_eq!(d.picked.len(), 7);
        // offered as the payment started and after the land
        assert_eq!(d.offered, vec![true, true]);
        assert!(!game.special_mana);
    }

    #[test]
    fn convoke_taps_creatures_for_mana() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Mountain", 10)]);
        game.step = Some(Step::PrecombatMain);
        game.priority = Some(0);
        let mountains: Vec<ObjId> = (0..2).map(|_| on_battlefield(&mut game, &db, "Mountain", 0)).collect();
        let bears: Vec<ObjId> = (0..2).map(|_| on_battlefield(&mut game, &db, "Bear Cub", 0)).collect();
        let stoke = game.create_obj(db.get("Stoke the Flames").unwrap(), 0, Zone::Hand);
        game.player_mut(0).hand.push(stoke);
        let mut d = SpecialPayer { picked: Vec::new(), offered: Vec::new() };
        // its target (the first thing offered), then {R}{R} from the lands and {2} from the two Bears
        assert!(cast(&mut game, &mut d, 0, stoke));
        assert!(mountains.iter().all(|m| game.obj(*m).tapped));
        assert!(bears.iter().all(|b| game.obj(*b).tapped));
    }

    #[test]
    fn improvise_taps_artifacts_for_generic_mana() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Swamp", 10)]);
        game.step = Some(Step::PrecombatMain);
        game.priority = Some(0);
        let swamp = on_battlefield(&mut game, &db, "Swamp", 0);
        let baubles: Vec<ObjId> = (0..6).map(|_| on_battlefield(&mut game, &db, "Mishra's Bauble", 0)).collect();
        let hauler = game.create_obj(db.get("Fen Hauler").unwrap(), 0, Zone::Hand);
        game.player_mut(0).hand.push(hauler);
        let mut d = SpecialPayer { picked: Vec::new(), offered: Vec::new() };
        assert!(cast(&mut game, &mut d, 0, hauler));
        assert!(game.obj(swamp).tapped);
        assert!(baubles.iter().all(|b| game.obj(*b).tapped));
        assert_eq!(game.obj(hauler).zone, Zone::Stack);
    }

    /// Keeps the looked-at cards each seat's view shows when it's asked to pick cards.
    struct LookWatcher(Vec<serde_json::Value>);

    impl Decider for LookWatcher {
        fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
            if matches!(query, Query::Cards { .. } | Query::Target { .. }) {
                self.0.push(crate::view::game_view(game, seat, &["a", "b"])["lookedAt"].clone());
                self.0.push(crate::view::game_view(game, 1 - seat, &["a", "b"])["lookedAt"].clone());
            }
            Passer.answer(game, seat, query)
        }
    }

    #[test]
    fn looked_at_cards_show_in_the_lookers_view_while_it_resolves() {
        let db = crate::cards();
        let mut game = game_with(&db, &[("Island", 10)]);
        let sleight = game.create_obj(db.get("Sleight of Hand").unwrap(), 0, Zone::Stack);
        Arc::make_mut(&mut game.stack).push(StackItem::spell(sleight, 0));
        let top: Vec<ObjId> = game.player(0).library.iter().take(2).copied().collect();
        let mut d = LookWatcher(Vec::new());
        resolve_top(&mut game, &mut d);
        assert!(d.0.len() >= 2);
        let mine = d.0[0].as_array().unwrap();
        assert_eq!(mine.len(), 1);
        assert!(mine[0]["name"].as_str().unwrap().starts_with("Sleight of Hand ["));
        assert_eq!(mine[0]["cards"].as_object().unwrap().len(), 2);
        assert_eq!(d.0[1], serde_json::json!([]));
        assert!(game.looked_at.is_empty());
        assert!(game.player(0).hand.contains(&top[0]) || game.player(0).hand.contains(&top[1]));
    }
}
