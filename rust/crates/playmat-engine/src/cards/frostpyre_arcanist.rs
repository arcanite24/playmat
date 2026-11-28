//! Frostpyre Arcanist: "When Frostpyre Arcanist enters, search your library for an instant or sorcery card with the
//! same name as a card in your graveyard, reveal it, put it into your hand, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("FrostpyreArcanist.FrostpyreArcanistPredicate", named_as_in_graveyard)];

/// a card in the searching player's graveyard has its name
fn named_as_in_graveyard(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let name = game.obj(id).name();
    game.player(you).graveyard.iter().any(|c| game.obj(*c).name() == name)
}
