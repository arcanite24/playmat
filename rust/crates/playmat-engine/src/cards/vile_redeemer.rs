//! Vile Redeemer: "When you cast this spell, you may pay {C}. If you do, create a 1/1 colorless Eldrazi Scion
//! creature token with "Sacrifice this token: Add {C}." for each nontoken creature that died under your control this
//! turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("VileRedeemer.VileRedeemerEffect", apply).tokens(&["EldraziScionToken"])];

/// As Java's `VileRedeemerNonTokenCreaturesDiedWatcher` counts: it looks the count up by the creature's id, not its
/// controller's, so each death sets that player's count to 1. One Scion if a nontoken creature died under your
/// control this turn, else none.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let died = game.watch.dies.iter().any(|(seat, _, creature, token)| *seat == you && *creature && !*token);
    if died {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    Some(())
}
