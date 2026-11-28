//! `LoseLifeControllerAttachedEffect(n)`: "enchanted creature's controller loses n life" (Sinister Possession).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LoseLifeControllerAttachedEffect", apply).args(&["amount"])];

/// the controller of what the source is attached to (as it last was there, `getLastKnownInformation`) loses the
/// amount; nothing when the source isn't attached
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (to, _) = game.obj(cx.source_id()).attached_to?;
    let who = game.obj(to).controller;
    let n = cx.arg_amount(game, 0)?;
    lose_life(game, who, n);
    Some(())
}
