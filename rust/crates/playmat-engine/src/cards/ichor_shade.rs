//! Ichor Shade: "At the beginning of your end step, if an artifact or creature was put into a graveyard from the
//! battlefield this turn, put a +1/+1 counter on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("IchorShade.IchorShadeCondition", artifact_or_creature_died)];

/// `IchorShadeWatcher`: a dies event of an artifact or creature this turn (tokens too)
fn artifact_or_creature_died(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.watch.dies.iter().any(|(_, artifact, creature, _)| *artifact || *creature)
}
