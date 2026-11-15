//! Die Young: "Choose target creature. You get {E}{E}, then you may pay any amount of {E}. The creature gets -1/-1
//! until end of turn for each {E} paid this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DieYoung.DieYoungEffect", apply)
    .effects(&["new GetEnergyCountersControllerEffect(2)"])];

/// {E}{E}; then `getAmount(0, energy)` (not asked with none); that much energy paid and as -X/-X until end of turn on
/// the target, if it's still there
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
            let e = EffectDef::BoostTargetEffect {
                power: crate::card::Amount::Fixed(-n),
                toughness: crate::card::Amount::Fixed(-n),
                duration: crate::card::Duration::EndOfTurn,
            };
            cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
        }
    }
    (!d.stopped()).then_some(())
}
