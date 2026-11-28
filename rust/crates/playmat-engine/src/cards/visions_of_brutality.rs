//! Visions of Brutality: "Whenever enchanted creature deals damage, its controller loses that much life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VisionsOfBrutality.VisionsOfBrutalityEffect", apply)];

/// the enchanted creature's controller (as it last was, `getPermanentOrLKIBattlefield`) loses the damage, X
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = game.obj(cx.source_id());
    let (to, _) = if aura.zone == Zone::Battlefield { aura.attached_to? } else { aura.lki_attached_to? };
    let seat = game.obj(to).controller;
    if cx.item.x > 0 {
        lose_life_by(game, seat, cx.item.x, Some(cx.source_id()), false);
    }
    Some(())
}
