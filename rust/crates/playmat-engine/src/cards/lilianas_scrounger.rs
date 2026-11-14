//! Liliana's Scrounger: "At the beginning of each end step, if a creature died this turn, you may put a loyalty
//! counter on a Liliana planeswalker you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilianasScrounger.LilianasScroungerEffect", apply)];

/// With a Liliana you control: the yes or no, then up to one of them (not targeted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you && game.has_type(*o, CardType::Planeswalker) && game.has_subtype(*o, "Liliana")
        })
        .collect();
    if possible.is_empty() || !choose_use(game, d, you, "Put a loyalty counter on a Liliana planeswalker you control?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&o) = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?.first() else { return Some(()) };
    add_counters_by(game, o, "loyalty", 1, Some(you));
    Some(())
}
