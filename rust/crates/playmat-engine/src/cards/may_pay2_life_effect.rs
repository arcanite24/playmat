//! `MayPay2LifeForColorAbility(color)`'s `MayPay2LifeEffect` (the Defilers): "As an additional cost to cast green
//! permanent spells, you may pay 2 life. Those spells cost {G} less to cast if you paid life this way. This effect
//! reduces only the amount of green mana you pay."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost("MayPay2LifeEffect", CostKind::MayPayLife(2), applies, reduce).args(&[
    "one_of:ObjectColor.WHITE|ObjectColor.BLUE|ObjectColor.BLACK|ObjectColor.RED|ObjectColor.GREEN",
])];

/// the color argument's place in WUBRG
fn color(cx: &StaticCx) -> Option<usize> {
    let c = cx.args.first()?.trim().strip_prefix("ObjectColor.")?;
    ["WHITE", "BLUE", "BLACK", "RED", "GREEN"].iter().position(|k| *k == c)
}

/// a permanent spell of the color, cast by the source's controller (`getCharacteristics`: the face cast)
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let Some(k) = color(cx) else { return false };
    let colors = if std::ptr::eq(of.face, &*game.obj(of.card).def) { game.colors(of.card) } else { of.face.colors.clone() };
    of.ability.is_none() && of.seat == cx.controller && of.face.is_permanent_card() && colors.iter().any(|c| *c as usize == k)
}

/// `CardUtil.adjustCost(spell, {G}, false)`: one symbol of the color, a colored one or else a hybrid one with it;
/// never generic mana
fn reduce(_: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let Some(k) = color(cx) else { return };
    if cost.colored[k] > 0 {
        cost.colored[k] -= 1;
        if cost.phyrexian[k] > cost.colored[k] {
            cost.phyrexian[k] = cost.colored[k];
        }
    } else if let Some(i) = cost.hybrid.iter().position(|(a, b)| *a == k || *b == k) {
        cost.hybrid.remove(i);
    }
}
