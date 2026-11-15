//! Balthier and Fran: "Whenever a Vehicle crewed by Balthier and Fran this turn attacks, if it's the first combat
//! phase of the turn, you may pay {1}{R}{G}. If you do, after this phase, there is an additional combat phase."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("BalthierAndFran.BalthierAndFranPredicate", crewed_by_source)];

/// `CrewedVehicleWatcher.checkIfCrewedThisTurn`: the source, still on the battlefield as the same object, crewed
/// this Vehicle (as it is now) this turn
fn crewed_by_source(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(source) = source else { return false };
    let s = game.obj(source);
    if s.zone != Zone::Battlefield {
        return false;
    }
    let (szcc, vzcc) = (s.zcc, game.obj(id).zcc);
    game.watch.crewed.iter().any(|(v, vz, c, cz)| *v == id && *vz == vzcc && *c == source && *cz == szcc)
}
