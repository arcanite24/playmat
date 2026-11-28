//! Avengers Assemble!: "At the beginning of each end step, if you attacked with a Hero this turn or a Hero entered
//! the battlefield under your control this turn, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("AvengersAssemble.AvengersAssembleCondition", hero_attacked_or_entered)];

/// a Hero: the permanent's subtypes while it's still that permanent, else its card's
fn hero(game: &Game, id: ObjId, zcc: u32) -> bool {
    let o = game.obj(id);
    if o.zone == Zone::Battlefield && o.zcc == zcc {
        game.has_subtype(id, "Hero")
    } else {
        o.def.subtypes.iter().any(|s| s == "Hero")
    }
}

/// `AttackedThisTurnWatcher` (the creatures that attacked this turn attacked for the active player) or
/// `PermanentsEnteredBattlefieldWatcher` (entered under your control)
fn hero_attacked_or_entered(game: &Game, you: Seat, _source: ObjId) -> bool {
    (game.active == Some(you) && game.attacked_this_turn.iter().any(|(id, zcc)| hero(game, *id, *zcc)))
        || game.watch.entered_permanents.iter().any(|(id, zcc, c)| *c == you && hero(game, *id, *zcc))
}
