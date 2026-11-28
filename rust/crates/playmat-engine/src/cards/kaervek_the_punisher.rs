//! Kaervek, the Punisher: "Whenever you commit a crime, exile up to one target black card from your graveyard and
//! copy it. You may cast the copy. If you do, you lose 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KaervekThePunisher.KaervekThePunisherEffect", apply)];

/// the target is exiled and copied (`game.copyCard`: a copy that leaves nothing behind once it's gone); the copy
/// is cast paying its costs (`controller.cast`: cancelling it is "you may"), and if it was, you lose 2 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    game.move_to(card, Zone::Exile);
    let def = game.obj(card).def.clone();
    let copy = game.create_obj(def, you, Zone::Exile);
    Arc::make_mut(&mut game.exile).push(copy);
    game.obj_mut(copy).token = true;
    let before = game.clone();
    if cast(game, d, you, copy) {
        lose_life_by(game, you, 2, Some(cx.source_id()), false);
    } else {
        *game = before;
    }
    Some(())
}
