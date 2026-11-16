//! Thief of Sanity: "Flying. Whenever Thief of Sanity deals combat damage to a player, look at the top three cards of
//! that player's library, exile one of them face down, then put the rest into their graveyard. For as long as that
//! card remains exiled, you may look at it, you may cast it, and you may spend mana as though it were mana of any type
//! to cast it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThiefOfSanity.ThiefOfSanityEffect", apply)];

/// `controller.choose(top three, TargetCard(LIBRARY))`; the one chosen is exiled face down, castable by you while it
/// stays exiled with mana of any type (`ExileFaceDownYouMayPlayAsLongAsExiledTargetEffect(true,
/// AS_THOUGH_ANY_MANA_TYPE)`, Gonti's); the rest still in the library go to the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let top: Vec<ObjId> = game.player(p).library.iter().take(3).copied().collect();
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
    for c in top {
        if game.obj(c).zone == Zone::Library {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}
