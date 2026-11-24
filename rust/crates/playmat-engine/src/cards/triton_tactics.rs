//! Triton Tactics: "Up to two target creatures each get +0/+3 until end of turn. Untap those creatures. At this turn's
//! next end of combat, tap each creature that was blocked by one of those creatures this turn and it doesn't untap
//! during its controller's next untap step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TritonTactics.TritonTacticsUntapEffect", untap_and_wait)
        .effects(&["new TritonTacticsTapEffect()"]),
    Handler::effect("TritonTactics.TritonTacticsTapEffect", tap_blocked)
        .effects(&["new DontUntapInControllersNextUntapStepTargetEffect()"]),
];

/// The targets still there are untapped; then a delayed trigger at this turn's `END_COMBAT_STEP_PRE`, fixed on
/// them (none once this turn's end of combat has begun: `Duration.EndOfTurn` ends it first)
fn untap_and_wait(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let objs: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    if objs.is_empty() {
        return Some(());
    }
    for o in &objs {
        untap_permanent(game, *o);
    }
    let before_end_of_combat = matches!(
        game.step,
        Some(
            Step::Untap
                | Step::Upkeep
                | Step::Draw
                | Step::PrecombatMain
                | Step::BeginCombat
                | Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::FirstCombatDamage
                | Step::CombatDamage
        )
    );
    if !before_end_of_combat {
        return Some(());
    }
    let delayed = crate::state::Delayed {
        rule: "At this turn's next end of combat, tap each creature that was blocked by one of those creatures this \
               turn and it doesn't untap during its controller's next untap step."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        // read from `watch.triton_tactics`, which keeps them once they're gone
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let untapped = objs.iter().map(|o| (*o, game.obj(*o).zcc)).collect();
    game.watch.triton_tactics.push((cx.source, untapped));
    game.at_end_of_combat
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// The creatures those (as they were when untapped, gone or not) blocked this turn, still on the battlefield as the
/// same objects, each tapped, then kept tapped through its controller's next untap step (`TritonTacticsWatcher`:
/// `BLOCKER_DECLARED` this turn)
fn tap_blocked(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let untapped: Vec<(ObjId, u32)> =
        game.watch.triton_tactics.iter().find(|(s, _)| *s == cx.source).map(|(_, u)| u.clone()).unwrap_or_default();
    let mut blocked: Vec<ObjId> = Vec::new();
    for b in &untapped {
        for (blocker, (a, za)) in &game.watch.blocks {
            let o = game.obj(*a);
            if blocker == b && o.zone == Zone::Battlefield && o.zcc == *za && !blocked.contains(a) {
                blocked.push(*a);
            }
        }
    }
    if blocked.is_empty() {
        return Some(());
    }
    for a in &blocked {
        tap(game, *a);
    }
    cx.run_on(game, d, cx.effects, blocked.into_iter().map(Target::Obj).collect());
    Some(())
}
