//! Swordsworn Cavalier: "Swordsworn Cavalier has first strike as long as another Knight entered the battlefield
//! under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("SwordswornCavalier.SwordswornCavalierWatcher.checkPermanent", another_knight_entered)];

/// `SwordswornCavalierWatcher`: the Knights that entered this turn, by the controller they entered under; one of
/// yours that isn't this permanent (`MageObjectReference.refersTo(source)`: the same object, zone-change count too).
/// A Knight is read as it is now while it's still that permanent, else as its card
fn another_knight_entered(game: &Game, you: Seat, source: ObjId) -> bool {
    let src_zcc = game.obj(source).zcc;
    game.watch
        .entered_permanents
        .iter()
        .any(|(id, zcc, seat)| *seat == you && !(*id == source && *zcc == src_zcc) && game.has_subtype(*id, "Knight"))
}
