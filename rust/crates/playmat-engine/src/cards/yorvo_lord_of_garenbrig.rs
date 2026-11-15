//! Yorvo, Lord of Garenbrig: "Yorvo enters with four +1/+1 counters on it. Whenever another green creature you
//! control enters, put a +1/+1 counter on Yorvo. Then if that creature's power is greater than Yorvo's power, put
//! another +1/+1 counter on Yorvo."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("YorvoLordOfGarenbrig.YorvoLordOfGarenbrigEffect", apply)];

/// a +1/+1 counter on Yorvo (still on the battlefield, or nothing); then another if the creature that entered (as it
/// is, or as it last was) has greater power than Yorvo now
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    put_counters(game, me, "+1/+1", 1, you);
    let Some(Target::Obj(o)) = cx.item.targets.first().and_then(|g| g.first()).copied() else { return Some(()) };
    let zcc = cx.item.target_zcc.first().and_then(|g| g.first()).copied();
    let power = if game.obj(o).zone == Zone::Battlefield && zcc.is_none_or(|z| z == game.obj(o).zcc) {
        game.power_toughness(o).0
    } else {
        let Some(p) = game.obj(o).power_lki else { return Some(()) };
        p
    };
    if power > game.power_toughness(me).0 {
        put_counters(game, me, "+1/+1", 1, you);
    }
    Some(())
}
