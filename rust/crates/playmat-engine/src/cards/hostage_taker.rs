//! Hostage Taker: "When Hostage Taker enters, exile another target artifact or creature until Hostage Taker leaves
//! the battlefield. You may cast that card for as long as it remains exiled, and you may spend mana as though it were
//! mana of any type to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HostageTaker.HostageTakerExileEffect", apply)];

/// Nothing once the Taker or the target is gone; the target is exiled until the Taker leaves (and then returns to the
/// battlefield, `OnLeaveReturnExiledAbility`), and you may cast it while it stays exiled, with mana of any type
/// (`CardUtil.makeCardPlayable(…, Duration.Custom, true)`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(t) = cx.obj(0) else { return Some(()) };
    if !cx.source_on_battlefield(game) || game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let (src, zcc) = cx.source;
    game.move_to(t, Zone::Exile);
    if game.obj(t).zone != Zone::Exile {
        return Some(());
    }
    game.exiled_until_leaves.push((src, zcc, t));
    let o = game.obj_mut(t);
    o.may_play = Some((you, u32::MAX));
    o.may_play_end_step = false;
    o.may_play_while = None;
    o.any_mana = true;
    o.cast_only = true;
    Some(())
}
