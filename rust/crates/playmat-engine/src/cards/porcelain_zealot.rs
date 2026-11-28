//! Porcelain Zealot: "At the beginning of combat on your turn, target creature you control gets +1/+1 until end of
//! turn. If that creature has toxic, instead it gets +2/+2 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PorcelainZealot.PorcelainZealotEffect", apply)];

/// a toxic ability, printed or granted
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let toxic = |a: &AbilityDef| matches!(a, AbilityDef::ToxicAbility { .. });
    let n = if game.obj(o).def.abilities.iter().any(toxic) || game.granted_abilities(o).iter().any(toxic) { 2 } else { 1 };
    boost(game, o, n, n, Duration::EndOfTurn);
    Some(())
}
