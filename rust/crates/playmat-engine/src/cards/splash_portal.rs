//! Splash Portal: "Exile target creature you control, then return it to the battlefield under its owner's control.
//! If that creature is a Bird, Frog, Otter, or Rat, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SplashPortal.SplashPortalCondition", small_creature)];

/// `game.getPermanent(source.getFirstTarget())`: the creature back on the battlefield (the same card), a Bird, Frog,
/// Otter or Rat
fn small_creature(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(Target::Obj(o)) = game.resolving_item.as_ref().and_then(|i| i.targets.first()?.first().copied()) else {
        return false;
    };
    game.obj(o).zone == crate::state::Zone::Battlefield
        && ["Bird", "Frog", "Otter", "Rat"].iter().any(|s| game.has_subtype(o, s))
}
