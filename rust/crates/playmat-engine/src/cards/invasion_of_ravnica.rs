//! Invasion of Ravnica: "When Invasion of Ravnica enters, exile target nonland permanent an opponent controls that
//! isn't exactly two colors." Guildpact Paragon (its back): "Whenever you cast a spell that's exactly two colors, look
//! at the top six cards of your library. You may reveal a card that's exactly two colors from among them and put it
//! into your hand. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("InvasionOfRavnica.InvasionOfRavnicaPredicate", not_two_colors),
    Handler::predicate("InvasionOfRavnica.GuildpactParagonPredicate", two_colors),
];

fn not_two_colors(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.colors(id).len() != 2
}

fn two_colors(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.colors(id).len() == 2
}
