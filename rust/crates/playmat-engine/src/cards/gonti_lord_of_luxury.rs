//! Gonti, Lord of Luxury: "When Gonti enters, look at the top four cards of target opponent's library, exile one of
//! them face down, then put the rest on the bottom of that library in a random order. You may cast that card for as
//! long as it remains exiled, and mana of any type can be spent to cast it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GontiLordOfLuxury.GontiLordOfLuxuryEffect", apply)];

/// `controller.choose(top four, TargetCard(LIBRARY))`; the one chosen is exiled face down, castable (not played) by
/// you while it stays exiled with mana of any type (`ExileFaceDownYouMayPlayAsLongAsExiledTargetEffect(true,
/// AS_THOUGH_ANY_MANA_TYPE)`); the rest go to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let top: Vec<ObjId> = game.player(p).library.iter().take(4).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &top, 1, 1, &rule)?;
    for c in &chosen {
        game.move_to(*c, Zone::Exile);
        if game.obj(*c).zone == Zone::Exile {
            let o = game.obj_mut(*c);
            o.may_play = Some((you, u32::MAX));
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.any_mana = true;
            o.cast_only = true;
            o.exiled_face_down = true;
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, p, &rest, false, false);
    Some(())
}
