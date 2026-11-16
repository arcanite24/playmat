//! Golbez, Crystal Collector: "At the beginning of your end step, if you control four or more artifacts, return
//! target creature card from your graveyard to your hand. Then if you control eight or more artifacts, each opponent
//! loses life equal to that card's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GolbezCrystalCollector.GolbezCrystalCollectorValue", card_power)];

/// the first target's card, wherever it is now: its power
fn card_power(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some(item) = game.resolving_item.as_ref() else { return 0 };
    let Some(Target::Obj(o)) = item.targets.first().and_then(|ts| ts.first()).copied() else { return 0 };
    game.power_toughness(o).0
}
