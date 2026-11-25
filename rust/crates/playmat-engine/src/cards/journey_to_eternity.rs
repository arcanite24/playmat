//! Journey to Eternity: "Enchant creature you control. When enchanted creature dies, return it to the battlefield
//! under your control, then return Journey to Eternity to the battlefield transformed under your control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("JourneyToEternity.JourneyToEternityReturnTransformedSourceEffect", apply)];

/// the source card, when it's in a public zone other than the battlefield (its graveyard, usually), enters
/// transformed under your control (`VALUE_KEY_ENTER_TRANSFORMED`, then `moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let me = cx.source_id();
    if !matches!(game.obj(me).zone, Zone::Graveyard | Zone::Exile) || game.obj(me).token {
        return Some(());
    }
    let Some(back) = game.obj(me).def.transform_back.clone() else { return Some(()) };
    let ob = game.obj_mut(me);
    ob.front = Some(std::mem::replace(&mut ob.def, back));
    enter_battlefield(game, d, me, cx.controller(), false);
    if d.stopped() {
        return None;
    }
    Some(())
}
