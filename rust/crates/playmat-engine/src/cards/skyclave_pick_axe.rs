//! Skyclave Pick-Axe: "Landfall — Whenever a land you control enters, equipped creature gets +2/+2 until end of
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkyclavePickAxe.SkyclavePickAxeEffect", apply)];

/// The creature it's attached to as this resolves (the Pick-Axe's last known one), while that's on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = game.obj(cx.source_id());
    let (o, zcc) = if src.zcc == cx.source.1 { src.attached_to } else { src.lki_attached_to }?;
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
        return None;
    }
    boost(game, o, 2, 2, crate::card::Duration::EndOfTurn);
    Some(())
}
