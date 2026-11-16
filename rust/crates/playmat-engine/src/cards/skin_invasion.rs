//! Skin Invasion: "Enchant creature. Enchanted creature attacks each combat if able. When enchanted creature dies,
//! return Skin Invasion to the battlefield transformed under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkinInvasion.SkinInvasionEffect", apply)];

/// `game.getCard(sourceId)` wherever it is (it went to the graveyard with the creature), onto the battlefield
/// transformed under the trigger's controller's control (`moveCards` with the enter-transformed value)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if matches!(game.obj(card).zone, Zone::Battlefield | Zone::Stack) || game.obj(card).token {
        return Some(());
    }
    let Some(back) = game.obj(card).def.transform_back.clone() else { return Some(()) };
    let o = game.obj_mut(card);
    o.front = Some(std::mem::replace(&mut o.def, back));
    enter_battlefield(game, d, card, cx.controller(), false);
    if d.stopped() {
        return None;
    }
    Some(())
}
