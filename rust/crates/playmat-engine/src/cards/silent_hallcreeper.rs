//! Silent Hallcreeper: "Silent Hallcreeper becomes a copy of another target creature you control." (one of its
//! combat damage modes)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SilentHallcreeper.SilentHallcreeperEffect", apply)];

/// `game.copyPermanent(Duration.Custom, …, EmptyCopyApplier)`: the target's copiable values, with no end, as long as
/// the Hallcreeper is still the permanent it was
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let of = cx.obj(0)?;
    if !cx.source_on_battlefield(game) || game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let copy = Arc::new((*game.obj(of).copiable()).clone());
    let o = game.obj_mut(cx.source_id());
    if o.printed_def.is_none() {
        o.printed_def = Some(o.def.clone());
    }
    o.copied_def = Some(copy.clone());
    o.def = copy;
    Some(())
}
