//! Warden of the Beyond: "Vigilance. Warden of the Beyond gets +2/+2 as long as an opponent owns a card in exile."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("WardenOfTheBeyond.WardenOfTheBeyondCondition", exiled)];

/// `Exile.getCardsInRange`: a card (not a token) in exile that an opponent owns
fn exiled(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.exile.iter().any(|c| game.obj(*c).owner != you && !game.obj(*c).token)
}
