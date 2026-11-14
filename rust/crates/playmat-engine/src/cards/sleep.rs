//! Sleep: "Tap all creatures target player controls. Those creatures don't untap during that player's next untap
//! step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Sleep.SleepEffect", apply)];

/// every creature the player controls is tapped, then all of them skip the next untap
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in &creatures {
        tap(game, *o);
    }
    if !creatures.is_empty() {
        let targets = creatures.iter().map(|o| Target::Obj(*o)).collect();
        cx.run_on(game, d, &[EffectDef::DontUntapInControllersNextUntapStepTargetEffect], targets);
    }
    Some(())
}
