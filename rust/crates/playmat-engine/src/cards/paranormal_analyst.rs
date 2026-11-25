//! Paranormal Analyst: "Whenever you manifest dread, put a card you put into your graveyard this way into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ParanormalAnalyst.ParanormalAnalystEffect", apply)];

/// The cards the manifest dread put into the graveyard (the trigger's `FixedTargets`), those still there: none does
/// nothing, one goes to hand, of more one is chosen (`TargetCardInYourGraveyard`, not targeted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let ids = cx.item.targets.first().map(|t| t.as_slice()).unwrap_or_default();
    let zccs = cx.item.target_zcc.first().map(|z| z.as_slice()).unwrap_or_default();
    let still: Vec<ObjId> = ids
        .iter()
        .zip(zccs)
        .filter_map(|(t, z)| match t {
            Target::Obj(o) if game.obj(*o).zone == Zone::Graveyard && game.obj(*o).zcc == *z => Some(*o),
            _ => None,
        })
        .collect();
    let card = match still.len() {
        0 => return Some(()),
        1 => still[0],
        _ => match choose_cards(game, d, you, &still, &still, 1, 1, "put into your hand") {
            Some(c) if !c.is_empty() => c[0],
            Some(_) => return Some(()),
            None => return None,
        },
    };
    game.move_to(card, Zone::Hand);
    Some(())
}
