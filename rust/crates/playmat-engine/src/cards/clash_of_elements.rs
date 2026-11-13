//! Clash of Elements: "Choose target nonland permanent. Its owner may put it on top of their library. If they do,
//! Clash of Elements deals 2 damage to them. If they didn't put the card on top of their library, they put it on
//! the bottom."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ClashOfElements.ClashOfElementsEffect", apply)];

/// The owner's `chooseUse` (Top or Bottom): on top and 2 damage to them, or on the bottom
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let owner = game.obj(o).owner;
    let message = format!(
        "Put {} on the top of your library and take 2 damage? (Otherwise on the bottom)",
        log::object(game, o)
    );
    if choose_use(game, d, owner, &message)? {
        if !put_on_library(game, d, owner, &[o], true, false) {
            return None;
        }
        deal_damage(game, cx.source_id(), Target::Player(owner), 2, false);
    } else if !put_on_library(game, d, owner, &[o], false, true) {
        return None;
    }
    Some(())
}
