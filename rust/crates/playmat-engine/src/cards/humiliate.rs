//! Humiliate: "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card.
//! Put a +1/+1 counter on a creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Humiliate.HumiliateEffect", apply)];

/// The counter's half: with a creature you control, you choose one (not targeted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&o) = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first() else { return Some(()) };
    add_counters_by(game, o, "+1/+1", 1, Some(you));
    Some(())
}
