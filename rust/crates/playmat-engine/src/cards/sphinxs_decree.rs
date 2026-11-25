//! Sphinx's Decree: "Each opponent can't cast instant or sorcery spells during that player's next turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SphinxsDecree.SphinxsDecreeEffect", apply)];

/// For each opponent, a `SphinxsDecreeCantCastEffect` (`CAST_SPELL` of an instant or sorcery) that applies only
/// during their next turn (`setStartingControllerAndTurnNum` with them as its "you", `isYourNextTurn`): the turn
/// after this one when it isn't theirs, the one after their own otherwise (two players, no extra turns)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for p in 0..game.seats() {
        if p == cx.controller() || game.player(p).lost {
            continue;
        }
        let next = if game.active == Some(p) { game.turn + game.seats() as u32 } else { game.turn + 1 };
        game.player_mut(p).no_instants_sorceries_turn = Some(next);
    }
    Some(())
}
