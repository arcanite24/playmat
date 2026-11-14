//! Chorale of the Void: "Whenever enchanted creature attacks, put target creature card from defending player's
//! graveyard onto the battlefield under your control tapped and attacking. Void — At the beginning of your end step,
//! sacrifice this Aura unless a nonland permanent left the battlefield this turn or a spell was warped this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("ChoraleOfTheVoid.ChoraleOfTheVoidPredicate", defending_owner)];

/// the card is owned by the defending player of the creature the source is attached to
fn defending_owner(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let Some(attacker) = game.attached(src) else { return false };
    game.obj(attacker).attacking.is_some_and(|p| game.obj(id).owner == p)
}
