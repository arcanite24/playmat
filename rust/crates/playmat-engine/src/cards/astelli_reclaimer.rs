//! Astelli Reclaimer: "When this creature enters, return target noncreature, nonland permanent card with mana value X
//! or less from your graveyard to the battlefield, where X is the amount of mana spent to cast this creature."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("AstelliReclaimer.AstelliReclaimerPredicate", at_most_mana_spent)];

/// its mana value is at most the mana spent to cast the source (`ManaPaidSourceWatcher.getTotalPaid`: 0 when it
/// wasn't cast)
fn at_most_mana_spent(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let spent = source.map_or(0, |s| if game.obj(s).cast { game.obj(s).mana_spent } else { 0 });
    game.obj(id).def.mana_value() <= spent
}
