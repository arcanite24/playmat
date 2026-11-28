//! Tifa's Limit Break: "Meteor Strikes — {2} — Double target creature's power and toughness until end of turn.
//! Final Heaven — {6}{G} — Triple target creature's power and toughness until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TifasLimitBreak.TifasLimitBreakEffect", apply)];

/// +(multiplier − 1) times its power and toughness as it resolves, until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let k = cx.arg_int(0)? - 1;
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let (p, t) = game.power_toughness(o);
    let e = EffectDef::BoostTargetEffect {
        power: crate::card::Amount::Fixed(k * p),
        toughness: crate::card::Amount::Fixed(k * t),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    Some(())
}
