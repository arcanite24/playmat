//! Heliod, the Radiant Dawn // Heliod, the Warped Eclipse: "When Heliod, the Radiant Dawn enters, return target
//! enchantment card that isn't a God from your graveyard to your hand. {3}{U/P}: Transform Heliod. Activate only as
//! a sorcery. // You may cast spells as though they had flash. Spells you cast cost {1} less to cast for each card
//! your opponents have drawn this turn."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "HeliodTheRadiantDawn.HeliodTheWarpedEclipseEffect",
    CostKind::Reduce,
    your_spell,
    less_per_card_drawn,
)];

/// a spell you cast
fn your_spell(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller
}

/// `CardsDrawnThisTurnWatcher`: {1} less for each card your opponents drew this turn (`CardUtil.adjustCost`)
fn less_per_card_drawn(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let n: u32 = (0..game.seats()).filter(|p| *p != cx.controller).map(|p| game.player(p).drawn).sum();
    cost.generic = cost.generic.saturating_sub(n as _);
}
