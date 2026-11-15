//! The Mycosynth Gardens: "{T}: Add {C}. {1}, {T}: Add one mana of any color. {X}, {T}: The Mycosynth Gardens
//! becomes a copy of target nontoken artifact you control with mana value X."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheMycosynthGardens.TheMycosynthGardensCopyEffect", apply)];

/// `game.copyPermanent(…, EmptyCopyApplier)` with no end: the target's copiable values, until the Gardens leave the
/// battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(of) = cx.obj(0) else { return Some(()) };
    if !cx.source_on_battlefield(game) || game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let src = cx.source_id();
    let copy = game.obj(of).copiable();
    let o = game.obj_mut(src);
    if o.printed_def.is_none() {
        o.printed_def = Some(o.def.clone());
    }
    o.copied_def = Some(copy.clone());
    o.def = copy;
    Some(())
}
