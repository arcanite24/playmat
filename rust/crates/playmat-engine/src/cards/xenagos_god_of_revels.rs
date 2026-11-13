//! Xenagos, God of Revels: "At the beginning of combat on your turn, another target creature you control gains haste
//! and gets +X/+X until end of turn, where X is that creature's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("XenagosGodOfRevels.XenagosGodOfRevelsEffect", apply)];

/// X is the target's power as this resolves, fixed from then on
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.power_toughness(o).0;
    let e = EffectDef::BoostTargetEffect {
        power: crate::card::Amount::Fixed(p),
        toughness: crate::card::Amount::Fixed(p),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    Some(())
}
