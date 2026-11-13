//! Indulgent Tormentor: "At the beginning of your upkeep, draw a card unless target opponent sacrifices a creature
//! or pays 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IndulgentTormentor.IndulgentTormentorEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])];

/// The opponent is asked about each cost they can pay, the sacrifice first; a cost asked about and paid ends it.
/// The sacrifice is `SacrificeTargetCost.pay`: a cancellable choice of one of their creatures.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = matching(game, &cx.filters[0], opponent, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == opponent)
        .collect();
    if !creatures.is_empty() && choose_use(game, d, opponent, "Sacrifice a creature to prevent the card draw?")? {
        let chosen = choose_cards_with(game, d, opponent, &creatures, &creatures, 1, 1, "sacrifice", true)?;
        if let Some(o) = chosen.first() {
            sacrifice(game, *o);
            return Some(());
        }
    }
    if game.player(opponent).life >= 3 && choose_use(game, d, opponent, "Pay 3 life to prevent the card draw?")? {
        lose_life(game, opponent, 3);
        return Some(());
    }
    draw(game, cx.controller(), 1);
    Some(())
}
