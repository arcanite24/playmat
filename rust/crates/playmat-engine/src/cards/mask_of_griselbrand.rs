//! Mask of Griselbrand: "Whenever equipped creature dies, you may pay X life, where X is its power. If you do, draw X
//! cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MaskOfGriselbrand.MaskOfGriselbrandEffect", apply)];

/// X is the creature's power as it last was on the battlefield (`getValue("attachedTo")`); when you can pay X life
/// you're asked, and paying it draws X
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(died) = died_equipped(game, cx) else { return Some(()) };
    let x = game.obj(died).power_lki.unwrap_or_else(|| game.obj(died).def.power);
    let cost = CostDef::PayLifeCost { amount: x };
    if !can_pay_cost(game, you, cx.source_id(), &cost) {
        return Some(());
    }
    let message = format!("Pay {} life? If you do, draw {} cards.", x, x);
    if choose_use(game, d, you, &message)? && pay_cost(game, d, you, cx.source_id(), &cost) {
        draw(game, you, x.max(0) as usize);
    }
    Some(())
}

/// The creature that died equipped with the Mask (`getValue("attachedTo")`): the trigger names no card, so it's the
/// object that left the battlefield with the Mask among its attachments, the one that left last when there are
/// several (the nearest the top of its graveyard; a token is in no zone list)
fn died_equipped(game: &Game, cx: &EffectCx) -> Option<ObjId> {
    let (mask, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
    (0..game.objs.len() as u32)
        .map(ObjId)
        .filter(|o| {
            let c = game.obj(*o);
            c.zone != Zone::Battlefield && c.lki_attachments.contains(&(mask, zcc))
        })
        .min_by_key(|o| {
            let c = game.obj(*o);
            let yard = &game.player(c.owner).graveyard;
            let from_top = yard.iter().rev().position(|g| g == o).unwrap_or(if c.token { 0 } else { usize::MAX });
            (from_top, std::cmp::Reverse(o.0))
        })
}
