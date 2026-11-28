//! `ManaValueLessThanControlledLandCountPredicate` (framework): "mana value less than or equal to the number of lands
//! you control", "you" being the filter's source's controller (`game.getControllerId(input.getSourceId())`).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("ManaValueLessThanControlledLandCountPredicate", mana_value_at_most_lands)];

fn mana_value_at_most_lands(game: &Game, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let owner = source.map_or(you, |s| game.obj(s).controller);
    let lands = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == owner && game.has_type(**b, CardType::Land))
        .count();
    game.obj(id).def.mana_value() as usize <= lands
}
