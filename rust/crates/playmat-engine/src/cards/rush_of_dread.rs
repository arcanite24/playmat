//! Rush of Dread: "Spree. + {1} — Target opponent sacrifices half the creatures they control, rounded up. + {2} —
//! Target opponent discards half the cards in their hand, rounded up. + {2} — Target opponent loses half their life,
//! rounded up."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("RushOfDread.RushOfDreadSacrificeEffect", sacrifice_half)
        .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"]),
    Handler::effect("RushOfDread.RushOfDreadDiscardEffect", discard_half),
];

/// the opponent's creatures counted, then `SacrificeEffect(FILTER_PERMANENT_CREATURE, half rounded up)` for them
fn sacrifice_half(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let creatures = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == opponent && game.has_type(**o, CardType::Creature))
        .count() as i32;
    let e = EffectDef::SacrificeEffect { filter: cx.filters[0].clone(), amount: Amount::Fixed((creatures + 1) / 2) };
    cx.run_on(game, d, &[e], vec![Target::Player(opponent)]);
    Some(())
}

/// half the opponent's hand, rounded up, discarded (`discard(n, false, false, …)`), a hand of one or more
fn discard_half(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let size = game.player(opponent).hand.len() as i32;
    if size == 0 {
        return Some(());
    }
    discard(game, d, opponent, (size + 1) / 2);
    if d.stopped() {
        return None;
    }
    Some(())
}
