//! `DontUntapInPlayersNextUntapStepAllEffect(filter)`: "during that player's next untap step, permanents of the
//! filter they control don't untap" (Imaginary Threats). The player is the effect's target.

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DontUntapInPlayersNextUntapStepAllEffect", apply).args(&["filter"])];

/// the target player's next untap step after this turn: their permanents of the filter, as they are then, stay
/// tapped (`dont_untap_next` with "you" for that player, used up by that untap step)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0) else { return Some(()) };
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let turn = game.turn;
    game.dont_untap_next.push((player, "you".to_string(), filter, cx.source_id(), turn));
    Some(())
}
