//! Huatli, Warrior Poet: "−X: Huatli deals X damage divided as you choose among any number of target creatures.
//! Creatures dealt damage this way can't block this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HuatliWarriorPoet.HuatliWarriorPoetDamageEffect", apply)
    .effects(&["new CantBlockTargetEffect(Duration.EndOfTurn)"])];

/// Each target still there dealt its share (`permanent.damage`, not combat); one dealt damage can't block this turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let chosen = cx.item.targets.first().cloned().unwrap_or_default();
    let ok = cx.legal.first().cloned().unwrap_or_default();
    for (t, n) in chosen.iter().zip(cx.item.target_amounts.iter()) {
        let Target::Obj(o) = t else { continue };
        if !ok.contains(t) || game.obj(*o).zone != Zone::Battlefield {
            continue;
        }
        if deal_damage(game, cx.source_id(), *t, *n, false) > 0 {
            cx.run_on(game, d, cx.effects, vec![*t]);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
