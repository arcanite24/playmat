//! Loot, Exuberant Explorer: "{4}{G}{G}, {T}: Look at the top six cards of your library. You may reveal a creature
//! card with mana value less than or equal to the number of lands you control from among them and put it onto the
//! battlefield. Put the rest on the bottom in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate(
    "LootExuberantExplorer.ManaValueLessThanControlledLandCountPredicate",
    at_most_your_lands,
)];

/// `ManaValueLessThanControlledLandCountPredicate`: its mana value is at most the number of lands the source's
/// controller controls
fn at_most_your_lands(game: &Game, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let you = source.map(|s| game.obj(s).controller).unwrap_or(you);
    let lands = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land))
        .count() as u32;
    game.obj(id).def.mana_value() <= lands
}
