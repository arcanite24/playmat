//! Primal Empathy: "At the beginning of your upkeep, draw a card if you control a creature with the greatest power
//! among creatures on the battlefield. Otherwise, put a +1/+1 counter on a creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PrimalEmpathy.PrimalEmpathyEffect", apply)];

/// The greatest power (0 with no creatures); one of yours at least that: the draw. Else `choose` a creature you
/// control (not targeted) for the counter.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    let highest = creatures.iter().map(|o| game.power_toughness(*o).0).max().unwrap_or(0);
    let yours: Vec<ObjId> = creatures.iter().copied().filter(|o| game.obj(*o).controller == you).collect();
    if yours.iter().any(|o| game.power_toughness(*o).0 >= highest) {
        draw(game, you, 1);
        return Some(());
    }
    if yours.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &yours, &yours, 1, 1, &rule)?;
    if let Some(&c) = chosen.first() {
        add_counters_by(game, c, "+1/+1", 1, Some(you));
    }
    Some(())
}
