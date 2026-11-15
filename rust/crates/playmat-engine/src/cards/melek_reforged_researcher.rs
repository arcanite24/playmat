//! Melek, Reforged Researcher: "Melek's power and toughness are each equal to twice the number of instant and
//! sorcery cards in your graveyard. The first instant or sorcery spell you cast each turn costs {3} less to cast."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("MelekReforgedResearcher.MelekReforgedResearcherPredicate", first_instant_or_sorcery)];

/// an instant or sorcery, while the player has cast no instant or sorcery spell this turn
/// (`MelekReforgedResearcherWatcher`)
fn first_instant_or_sorcery(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    (game.obj(id).is(CardType::Instant) || game.obj(id).is(CardType::Sorcery))
        && game.player(you).instants_sorceries_cast == 0
}
