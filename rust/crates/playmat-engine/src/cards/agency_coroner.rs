//! Agency Coroner: "{2}{B}, Sacrifice another creature: Draw a card. If the sacrificed creature was suspected, draw
//! two cards instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("AgencyCoroner.AgencyCoronerCondition", sacrificed_suspected)];

/// the resolving ability's `SacrificeTargetCost` permanents: any of them suspected as it was sacrificed
fn sacrificed_suspected(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(item) = game.resolving_item.as_ref().or(game.checking_item.as_ref()) else { return false };
    item.sacrificed.iter().any(|o| game.obj(*o).suspected_lki)
}
