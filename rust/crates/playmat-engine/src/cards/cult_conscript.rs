//! Cult Conscript: "This creature enters tapped. {1}{B}: Return this card from your graveyard to the battlefield.
//! Activate only if a non-Skeleton creature died under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("CultConscript.CultConscriptCondition", non_skeleton_died)];

/// `CultConscriptWatcher`: a creature that wasn't a Skeleton died under your control this turn
fn non_skeleton_died(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch
        .left_battlefield
        .iter()
        .any(|l| l.died && l.creature && l.controller == you && !l.subtypes.iter().any(|t| t == "Skeleton"))
}
