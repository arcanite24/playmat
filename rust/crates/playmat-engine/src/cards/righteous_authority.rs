//! Righteous Authority: "Enchanted creature gets +1/+1 for each card in its controller's hand."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("RighteousAuthority.CardsInEnchantedControllerHandCount", in_hand)];

/// the hand of the enchanted creature's controller, 0 when it's attached to nothing on the battlefield
fn in_hand(game: &Game, _you: Seat, source: ObjId, _x: i32) -> i32 {
    if game.obj(source).zone != Zone::Battlefield {
        return 0;
    }
    match game.obj(source).attached_to {
        Some((c, _)) if game.obj(c).zone == Zone::Battlefield => {
            game.player(game.obj(c).controller).hand.len() as i32
        }
        _ => 0,
    }
}
