//! Terror of the Peaks: "Spells your opponents cast that target Terror of the Peaks cost an additional 3 life to
//! cast. Whenever another creature you control enters, Terror of the Peaks deals damage equal to that creature's
//! power to any target."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::cost("TerrorOfThePeaks.TerrorOfThePeaksCostIncreaseEffect", CostKind::Life(3), applies, none),
    Handler::value("TerrorOfThePeaks.TerrorOfThePeaksValue", entering_power),
];

/// An opponent's spell being cast with this among its chosen targets (`CardUtil.getAllSelectedTargets`). While
/// what's playable is checked, Java only adds the cost when every possible target is this one, so the spell can't
/// be cast without paying it; the engine doesn't add it then (the spell isn't on the stack yet).
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && cx.opponent(of.seat)
        && game.stack.iter().rev().find(|i| i.card == of.card && i.ability.is_none()).is_some_and(|i| {
            i.targets.iter().flatten().any(|t| *t == Target::Obj(cx.source))
        })
}

fn none(_: &Game, _: &StaticCx, _: &CostCx, _: &mut crate::card::ManaCost) {}

/// `permanentEnteringBattlefield`'s power: the trigger's entering creature, as it is now (as it last was, once it
/// has left)
fn entering_power(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some(e) = game.resolving_item.as_ref().and_then(|i| i.entering) else { return 0 };
    if game.obj(e).zone == Zone::Battlefield {
        game.power_toughness(e).0
    } else {
        game.obj(e).power_lki.unwrap_or(0)
    }
}
