//! The rules: turns and steps, priority, playing lands and casting spells, paying mana, resolving, combat,
//! state-based actions. The order of everything a player sees follows Java's `GameImpl.playPriority` and
//! `HumanPlayer` with the client's default settings, so a shadow game replays answer for answer.

use crate::card::{AbilityDef, CardType, Color, EffectDef, Mana, ManaCost, TargetDef};
use crate::decide::{Answer, Decider, Query};
use crate::state::{Game, ObjId, Seat, StackItem, Step, Target, Zone};
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
                game.player_mut(s).lands_played = 0;
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
        Step::DeclareAttackers => declare_attackers(game, d, active),
        Step::DeclareBlockers => declare_blockers(game, d, game.opponent(active)),
        Step::FirstCombatDamage => combat_damage(game, true),
        Step::CombatDamage => combat_damage(game, false),
        Step::Cleanup => cleanup(game, d, active),
        _ => {}
    }
}

fn end_step(game: &mut Game, step: Step) {
    for s in 0..game.seats() {
        game.player_mut(s).mana_pool = Mana::default();
    }
    if step == Step::EndCombat {
        for id in Arc::clone(&game.battlefield).iter().copied() {
            let o = game.obj(id);
            if o.attacking.is_some() || !o.blocking.is_empty() || o.blocked {
                let o = game.obj_mut(id);
                o.attacking = None;
                o.blocking.clear();
                o.blocked = false;
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
                state_based_actions(game);
                if game.over {
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
        resolve_top(game);
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
    out
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

fn has_targets(game: &Game, seat: Seat, id: ObjId) -> bool {
    game.obj(id).def.spell.targets.iter().all(|t| {
        let (players, objs) = possible_targets(game, seat, t);
        !players.is_empty() || !objs.is_empty()
    })
}

/// Untapped permanents that can make mana now, with the mana each makes.
fn mana_sources(game: &Game, seat: Seat) -> Vec<(ObjId, Mana)> {
    let mut out = Vec::new();
    for id in game.battlefield.iter() {
        let o = game.obj(*id);
        if o.controller != seat || o.tapped {
            continue;
        }
        if o.is(CardType::Creature) && o.sick && !o.has(&AbilityDef::HasteAbility) {
            continue;
        }
        if let Some(m) = o.def.abilities.iter().find_map(|a| a.mana()) {
            out.push((*id, m));
        }
    }
    out
}

/// Whether the pool and untapped sources (one mana each) can pay a cost.
fn can_pay(game: &Game, seat: Seat, cost: &ManaCost) -> bool {
    let mut avail: Vec<Mana> = mana_sources(game, seat).into_iter().map(|(_, m)| m).collect();
    let pool = game.player(seat).mana_pool;
    for c in Color::ALL {
        for _ in 0..pool.colored[c.index()] {
            avail.push(Mana::of(c, 1));
        }
    }
    for _ in 0..pool.colorless {
        avail.push(Mana { colorless: 1, ..Mana::default() });
    }
    let mut used = vec![false; avail.len()];
    for c in Color::ALL {
        for _ in 0..cost.colored[c.index()] {
            match (0..avail.len()).find(|i| !used[*i] && avail[*i].colored[c.index()] > 0) {
                Some(i) => used[i] = true,
                None => return false,
            }
        }
    }
    for _ in 0..cost.colorless {
        match (0..avail.len()).find(|i| !used[*i] && avail[*i].colorless > 0) {
            Some(i) => used[i] = true,
            None => return false,
        }
    }
    used.iter().filter(|u| !**u).count() as u32 >= cost.generic
}

fn play(game: &mut Game, d: &mut dyn Decider, seat: Seat, id: ObjId) {
    if game.obj(id).is(CardType::Land) {
        game.move_to(id, Zone::Battlefield);
        game.obj_mut(id).controller = seat;
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
    game.obj_mut(id).zone = Zone::Stack;
    Arc::make_mut(&mut game.stack).push(StackItem { card: id, controller: seat, targets: Vec::new() });
    let defs = game.obj(id).def.spell.targets.clone();
    let mut targets = Vec::new();
    for t in &defs {
        match choose_target(game, d, seat, t) {
            Some(target) => targets.push(target),
            None => return false,
        }
    }
    if let Some(top) = Arc::make_mut(&mut game.stack).last_mut() {
        top.targets = targets;
    }
    let cost = game.obj(id).def.cost.clone();
    for s in 0..game.seats() {
        game.player_mut(s).passed = false;
    }
    pay_mana(game, d, seat, &cost)
}

/// `HumanPlayer.chooseTarget` for a spell's target. Auto-target level 1 picks the only legal target when it's a
/// permanent or yourself, never another player. A cast can be cancelled (a boolean); other answers are ignored.
fn choose_target(game: &mut Game, d: &mut dyn Decider, seat: Seat, def: &TargetDef) -> Option<Target> {
    let (players, objs) = possible_targets(game, seat, def);
    if players.len() + objs.len() == 1 {
        if let Some(o) = objs.first() {
            return Some(Target::Obj(*o));
        }
        if players[0] == seat {
            return Some(Target::Player(seat));
        }
    }
    loop {
        let q = Query::Target { players: players.clone(), objs: objs.clone(), required: false };
        match ask(game, d, seat, &q) {
            Answer::Player(s) if players.contains(&s) => return Some(Target::Player(s)),
            Answer::Obj(o) if objs.contains(&o) => return Some(Target::Obj(o)),
            Answer::Bool(_) | Answer::Stop => return None,
            _ => {}
        }
    }
}

pub fn possible_targets(game: &Game, seat: Seat, def: &TargetDef) -> (Vec<Seat>, Vec<ObjId>) {
    let creatures = || -> Vec<ObjId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|id| {
                let o = game.obj(*id);
                o.is(CardType::Creature) && !(o.controller != seat && o.has(&AbilityDef::HexproofAbility))
            })
            .collect()
    };
    let players: Vec<Seat> = (0..game.seats()).filter(|s| !game.player(*s).lost).collect();
    match def {
        TargetDef::TargetAnyTarget => (players, creatures()),
        TargetDef::TargetCreaturePermanent => (Vec::new(), creatures()),
        TargetDef::TargetPlayer => (players, Vec::new()),
        TargetDef::TargetOpponent => (players.into_iter().filter(|s| *s != seat).collect(), Vec::new()),
    }
}

fn target_legal(game: &Game, seat: Seat, def: &TargetDef, t: Target) -> bool {
    let (players, objs) = possible_targets(game, seat, def);
    match t {
        Target::Player(s) => players.contains(&s),
        Target::Obj(o) => objs.contains(&o),
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
                if let Some((_, m)) = mana_sources(game, seat).into_iter().find(|(s, _)| *s == id) {
                    game.obj_mut(id).tapped = true;
                    let pool = &mut game.player_mut(seat).mana_pool;
                    for c in Color::ALL {
                        pool.colored[c.index()] += m.colored[c.index()];
                    }
                    pool.colorless += m.colorless;
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

fn resolve_top(game: &mut Game) {
    let item = match Arc::make_mut(&mut game.stack).pop() {
        Some(i) => i,
        None => return,
    };
    let id = item.card;
    let def = game.obj(id).def.clone();
    if def.is_permanent_card() {
        game.move_to(id, Zone::Battlefield);
        game.obj_mut(id).controller = item.controller;
        return;
    }
    let legal: Vec<bool> = def
        .spell
        .targets
        .iter()
        .zip(item.targets.iter())
        .map(|(t, x)| target_legal(game, item.controller, t, *x))
        .collect();
    if def.spell.targets.is_empty() || legal.iter().any(|l| *l) {
        for e in &def.spell.effects {
            match e {
                EffectDef::DamageTargetEffect { amount } => {
                    if let (Some(t), Some(true)) = (item.targets.first(), legal.first()) {
                        deal_damage(game, id, *t, *amount, false);
                    }
                }
                EffectDef::DrawCardSourceControllerEffect { amount } => draw(game, item.controller, *amount as usize),
                EffectDef::GainLifeEffect { amount } => game.player_mut(item.controller).life += amount,
            }
        }
    }
    game.move_to(id, Zone::Graveyard);
}

/// Damage from `source`: to a player it's life lost, to a creature it's marked. Lifelink gains its controller life.
fn deal_damage(game: &mut Game, source: ObjId, to: Target, amount: i32, combat: bool) {
    if amount <= 0 {
        return;
    }
    let _ = combat;
    let src = game.obj(source).clone();
    match to {
        Target::Player(s) => game.player_mut(s).life -= amount,
        Target::Obj(o) => {
            if game.obj(o).zone != Zone::Battlefield {
                return;
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
        game.player_mut(src.controller).life += amount;
    }
}

// ------------------------------------------------------------------------------------------------------------ combat

pub fn attackers(game: &Game) -> Vec<ObjId> {
    game.battlefield.iter().copied().filter(|id| game.obj(*id).attacking.is_some()).collect()
}

fn first_strike_in_combat(game: &Game) -> bool {
    game.battlefield.iter().any(|id| {
        let o = game.obj(*id);
        (o.attacking.is_some() || !o.blocking.is_empty())
            && (o.has(&AbilityDef::FirstStrikeAbility) || o.has(&AbilityDef::DoubleStrikeAbility))
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
    let b = game.obj(blocker);
    let a = game.obj(attacker);
    if a.has(&AbilityDef::FlyingAbility) && !b.has(&AbilityDef::FlyingAbility) && !b.has(&AbilityDef::ReachAbility) {
        return false;
    }
    true
}

/// `HumanPlayer.selectBlockers`: a creature, then the attacker it blocks (picked for you when there's one); done ends.
fn declare_blockers(game: &mut Game, d: &mut dyn Decider, seat: Seat) {
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
    let o = game.obj(id);
    let fs = o.has(&AbilityDef::FirstStrikeAbility);
    let ds = o.has(&AbilityDef::DoubleStrikeAbility);
    if first {
        fs || ds
    } else {
        !has_first_step || ds || !fs
    }
}

/// Combat damage, all at once: attackers to their blockers in order (lethal each, the rest to the last, or with
/// trample to the player), unblocked attackers to the player, blockers to the attacker they block.
fn combat_damage(game: &mut Game, first: bool) {
    let has_first_step = first || game.step == Some(Step::CombatDamage) && first_strike_happened(game);
    let mut hits: Vec<(ObjId, Target, i32)> = Vec::new();
    for a in attackers(game) {
        if !deals_damage_in(game, a, first, has_first_step) {
            continue;
        }
        let (power, _) = game.power_toughness(a);
        let defender = match game.obj(a).attacking {
            Some(s) => s,
            None => continue,
        };
        let blockers: Vec<ObjId> =
            game.battlefield.iter().copied().filter(|b| game.obj(*b).blocking.contains(&a)).collect();
        if !game.obj(a).blocked {
            hits.push((a, Target::Player(defender), power));
            continue;
        }
        let trample = game.obj(a).has(&AbilityDef::TrampleAbility);
        let deathtouch = game.obj(a).has(&AbilityDef::DeathtouchAbility);
        let mut left = power;
        for (i, b) in blockers.iter().enumerate() {
            let (_, t) = game.power_toughness(*b);
            let lethal = if deathtouch { 1 } else { (t - game.obj(*b).damage).max(0) };
            let last = i + 1 == blockers.len();
            let n = if last && !trample { left } else { lethal.min(left) };
            hits.push((a, Target::Obj(*b), n));
            left -= n;
        }
        if trample && left > 0 {
            hits.push((a, Target::Player(defender), left));
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
    for (src, to, n) in hits {
        deal_damage(game, src, to, n, true);
    }
}

fn first_strike_happened(game: &Game) -> bool {
    first_strike_in_combat(game)
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
                    || (!o.has(&AbilityDef::IndestructibleAbility)
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
        let shock = game.create_obj(db.get("Shock").unwrap(), 0, Zone::Hand);
        game.player_mut(0).hand.push(shock);
        Arc::make_mut(&mut game.stack).push(StackItem { card: shock, controller: 0, targets: vec![Target::Obj(bear)] });
        game.remove_from_zone(shock);
        Arc::make_mut(&mut game.stack).push(StackItem { card: shock, controller: 0, targets: vec![Target::Obj(bear)] });
        Arc::make_mut(&mut game.stack).remove(0);
        resolve_top(&mut game);
        state_based_actions(&mut game);
        assert_eq!(game.player(1).graveyard, vec![bear]);
        assert_eq!(game.player(0).graveyard, vec![shock]);
    }
}
