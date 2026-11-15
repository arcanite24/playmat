//! Harnessed Lightning: "Choose target creature. You get {E}{E}{E}, then you may pay any amount of {E}. Harnessed
//! Lightning deals that much damage to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HarnessedLightning.HarnessedLightningEffect", apply)
    .effects(&["new GetEnergyCountersControllerEffect(3)"])];

/// {E}{E}{E}; then `getAmount(0, energy)` (not asked with none); that much energy paid and as damage to the target,
/// if it's still there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, cx.effects);
    if d.stopped() {
        return None;
    }
    let max = energy(game, you);
    let n = if max <= 0 {
        0
    } else {
        loop {
            match ask(game, d, you, &Query::Amount { min: 0, max }) {
                Answer::Int(n) if (0..=max).contains(&n) => break n,
                Answer::Stop => return None,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        }
    };
    if n > 0 {
        add_energy(game, you, -n);
        if let Some(o) = cx.obj(0) {
            deal_damage(game, cx.source_id(), Target::Obj(o), n, false);
        }
    }
    Some(())
}
