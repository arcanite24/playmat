//! Veteran Survivor: "Survival — At the beginning of your second main phase, if Veteran Survivor is tapped, exile up
//! to one target card from a graveyard. As long as there are three or more cards exiled with Veteran Survivor, it
//! gets +3/+3 and has hexproof."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("VeteranSurvivor.VeteranSurvivorCondition", three_exiled)];

/// the cards still in its exile zone (`CardUtil.getExileZoneId` for the Survivor as it is now) are three or more
fn three_exiled(game: &Game, _: Seat, source: ObjId) -> bool {
    let zcc = game.obj(source).zcc;
    game.exiled_for_source.iter().filter(|(s, z, c)| *s == source && *z == zcc && game.obj(*c).zone == Zone::Exile).count()
        >= 3
}
