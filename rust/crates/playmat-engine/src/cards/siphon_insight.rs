//! Siphon Insight: "Look at the top two cards of target opponent's library. Exile one of them face down and put the
//! other on the bottom of that library. You may look at and play the exiled card for as long as it remains exiled,
//! and you may spend mana as though it were mana of any color to cast that spell. Flashback {1}{U}{B}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SiphonInsight.SiphonInsightEffect", apply)];

/// `controller.choose(top two, TargetCard(LIBRARY))`; the one chosen is exiled face down, playable by you while it
/// stays exiled with mana as though of any color (`ExileFaceDownYouMayPlayAsLongAsExiledTargetEffect(false,
/// AS_THOUGH_ANY_MANA_COLOR)`: the engine's any-mana flag); the rest go to the bottom
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let top: Vec<ObjId> = game.player(p).library.iter().take(2).copied().collect();
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
            o.cast_only = false;
            o.exiled_face_down = true;
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false);
    Some(())
}
