//! Hydra's Growth: "At the beginning of your upkeep, double the number of +1/+1 counters on enchanted creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HydrasGrowth.HydrasGrowthDoubleEffect", apply)];

/// The Aura as it is or last was on the battlefield, and the creature it's attached to
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = game.obj(cx.source_id());
    let to = if aura.zone == Zone::Battlefield { aura.attached_to } else { aura.lki_attached_to };
    let Some((o, zcc)) = to else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
        return Some(());
    }
    let n = game.obj(o).counter("+1/+1");
    if n > 0 {
        add_counters_by(game, o, "+1/+1", n, Some(cx.controller()));
    }
    Some(())
}
