//! Ride Down: "Destroy target blocking creature. Creatures that were blocked by that creature this combat gain
//! trample until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RideDown.RideDownEffect", apply)
    .effects(&["new GainAbilityTargetEffect(TrampleAbility.getInstance(), Duration.EndOfTurn)"])];

/// Trample for the attackers it blocks, then the destroy
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let attackers = game.obj(o).blocking.clone();
    if !attackers.is_empty() {
        let targets: Vec<Target> = attackers.iter().map(|a| Target::Obj(*a)).collect();
        cx.run_on(game, d, &cx.effects[..1], targets);
    }
    destroy(game, o);
    Some(())
}
