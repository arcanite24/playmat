//! Kemba, Kha Enduring: "Whenever Kemba or another Cat you control enters, attach up to one target Equipment you
//! control to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KembaKhaEnduring.KembaKhaEnduringEffect", apply)];

/// `Permanent.addAttachment`: the Equipment onto the Cat that entered, both still on the battlefield
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cat = cx.item.entering?;
    let equipment = cx.obj(0)?;
    if game.obj(cat).zone != Zone::Battlefield || equipment == cat {
        return Some(());
    }
    let zcc = game.obj(cat).zcc;
    game.attach_to(equipment, cat, zcc);
    Some(())
}
