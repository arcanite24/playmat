//! Starving Revenant: "When Starving Revenant enters the battlefield, surveil 2. Then for each card you put on top of
//! your library, you draw a card and you lose 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StarvingRevenant.StarvingRevenantEffect", apply)];

/// `doSurveil(2)` as the engine's surveil does it; then for each card kept on top, you draw a card and lose 3 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
    let mut on_top = 0;
    if !cards.is_empty() {
        let rule = item_rule(game, cx.item);
        let yard = choose_cards(game, d, you, &cards, &cards, 0, cards.len(), &rule)?;
        for c in &yard {
            game.move_to(*c, Zone::Graveyard);
        }
        let top: Vec<ObjId> = cards.iter().copied().filter(|c| !yard.contains(c)).collect();
        if !put_on_library(game, d, you, &top, true, true) {
            return None;
        }
        on_top = top.len();
    }
    scried(game, you, 2, false);
    for _ in 0..on_top {
        draw(game, you, 1);
        lose_life(game, you, 3);
    }
    Some(())
}
