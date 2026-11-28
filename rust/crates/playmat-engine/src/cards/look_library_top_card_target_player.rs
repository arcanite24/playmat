//! `LookLibraryTopCardTargetPlayerEffect(amount[, putToGraveyard])` (Draugr Thought-Thief): "look at the top card
//! of target player's library. You may put that card into their graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LookLibraryTopCardTargetPlayerEffect", apply)
    .args(&["int", "bool", "one_of:false"])];

/// You look at the top `amount` cards of the target player's library; with `putToGraveyard`, for each "Put that card
/// into its owner's graveyard?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let amount = cx.arg_int(0).unwrap_or(1).max(0) as usize;
    let to_graveyard = cx.args.get(1).is_some_and(|a| a.trim() == "true");
    let cards: Vec<ObjId> = game.player(p).library.iter().take(amount).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let src = cx.source_id();
    let zcc = game.obj(src).zcc;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source: src, zcc, bare: true, cards: cards.clone() });
    if to_graveyard {
        for c in cards {
            if choose_use(game, d, you, "Put that card into its owner's graveyard?")? {
                game.move_to(c, Zone::Graveyard);
            }
        }
    }
    Some(())
}
