//! Kiora of Salt and Sand: "Whenever you attack, if you've activated a loyalty ability this turn, untap target
//! attacking creature. It can't be blocked this turn. Planeswalkers you control have "[-8]: Create an 8/8 blue
//! Leviathan creature token with hexproof.""

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("KioraOfSaltAndSand.KioraOfSaltAndSandCondition", activated_loyalty)];

/// `KioraOfSaltAndSandWatcher`: you activated a loyalty ability this turn (`ACTIVATED_ABILITY` of a
/// `LoyaltyAbility`)
fn activated_loyalty(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.loyalty_activated.contains(&you)
}
