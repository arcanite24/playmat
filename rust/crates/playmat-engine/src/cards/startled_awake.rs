//! Startled Awake: "{3}{U}{U}: Put Startled Awake from your graveyard onto the battlefield transformed. Activate only
//! as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StartledAwake.StartledAwakeReturnTransformedEffect", apply)];

/// While the card is still in the graveyard (`getSourceCardIfItStillExists`, then its zone): its back face enters
/// under you
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    super::ojer_taq_deepest_foundation::return_transformed(game, d, cx, false)
}
