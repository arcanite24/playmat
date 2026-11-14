//! Moseo, Vein's New Dean: "Infusion — At the beginning of your end step, if you gained life this turn, return up
//! to one target creature card with mana value X or less from your graveyard to the battlefield, where X is the
//! amount of life you gained this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("MoseoVeinsNewDean.MoseoVeinsNewDeanPredicate", at_most_life_gained)];

/// Its mana value is at most the life the source's controller gained this turn (`ControllerGainedLifeCount`)
fn at_most_life_gained(game: &Game, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let you = source.map(|s| game.obj(s).controller).unwrap_or(you);
    (game.obj(id).def.mana_value() as i32) <= game.player(you).life_gained
}
