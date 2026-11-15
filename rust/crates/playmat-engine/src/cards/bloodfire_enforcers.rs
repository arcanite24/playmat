//! Bloodfire Enforcers: "This creature has first strike and trample as long as an instant card and a sorcery card are
//! in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("BloodfireEnforcers.BloodfireEnforcersCondition", instant_and_sorcery)];

/// your graveyard has an instant card and a sorcery card (an instant sorcery card counts as the instant)
fn instant_and_sorcery(game: &Game, you: Seat, _: ObjId) -> bool {
    let (mut instant, mut sorcery) = (false, false);
    for c in game.player(you).graveyard.iter().copied() {
        if game.has_type(c, CardType::Instant) {
            if sorcery {
                return true;
            }
            instant = true;
        } else if game.has_type(c, CardType::Sorcery) {
            if instant {
                return true;
            }
            sorcery = true;
        }
    }
    false
}
