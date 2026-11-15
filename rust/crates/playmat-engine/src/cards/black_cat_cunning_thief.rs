//! Black Cat, Cunning Thief: "When Black Cat enters, look at the top nine cards of target opponent's library, exile
//! two of them face down, then put the rest on the bottom of their library in a random order. You may play the exiled
//! cards for as long as they remain exiled. Mana of any type can be spent to cast spells this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlackCatCunningThief.BlackCatCunningThiefEffect", apply)];

/// `controller.choose(top nine, TargetCard(2, 2))`; the two chosen are exiled, and you may play them while they stay
/// exiled, with mana of any type (`ExileFaceDownYouMayPlayAsLongAsExiledTargetEffect`: face down, so choosing one first
/// asks whether to only look at it); the rest go to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let top: Vec<ObjId> = game.player(p).library.iter().take(9).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let n = top.len().min(2);
    let chosen = choose_cards(game, d, you, &top, &top, n, n, &rule)?;
    for c in &chosen {
        game.move_to(*c, Zone::Exile);
        if game.obj(*c).zone == Zone::Exile {
            let o = game.obj_mut(*c);
            o.may_play = Some((you, u32::MAX));
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.any_mana = true;
            o.exiled_face_down = true;
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, p, &rest, false, false);
    Some(())
}
