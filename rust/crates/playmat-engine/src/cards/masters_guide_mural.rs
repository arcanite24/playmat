//! Master's Guide-Mural // Master's Manufactory: "When Master's Guide-Mural enters, create a 4/4 white and blue Golem
//! artifact creature token. Craft with artifact {4}{W}{W}{U}." // "{T}: Create a 4/4 white and blue Golem artifact
//! creature token. Activate only if Master's Manufactory or another artifact entered the battlefield under your
//! control this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("MastersGuideMural.MastersManufactoryCondition", condition)];

/// `MastersManufactoryWatcher.check`: the source is on the battlefield, and an artifact entered the battlefield under
/// your control this turn (the Manufactory itself is one), counting those that have left since (each object keeps
/// the turn it last entered and who controlled it)
fn condition(game: &Game, you: Seat, source: ObjId) -> bool {
    if game.obj(source).zone != Zone::Battlefield {
        return false;
    }
    let turn = game.turn;
    game.objs.iter().enumerate().any(|(i, o)| {
        o.entered_turn == turn
            && o.controller == you
            && if o.zone == Zone::Battlefield {
                game.has_type(ObjId(i as u32), CardType::Artifact)
            } else {
                o.is(CardType::Artifact)
            }
    })
}
