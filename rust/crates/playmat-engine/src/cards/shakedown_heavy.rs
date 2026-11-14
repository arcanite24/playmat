//! Shakedown Heavy: "Menace. Whenever Shakedown Heavy attacks, defending player may have you draw a card. If they
//! do, untap Shakedown Heavy and remove it from combat."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShakedownHeavy.ShakedownHeavyEffect", apply)];

/// The defending player (the trigger's target pointer) is asked; yes: you draw, then the source, still the same
/// permanent, untaps and leaves combat
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let you = cx.controller();
    let message = format!("Have {} draw a card?", log::player(you));
    if !choose_use(game, d, p, &message)? {
        return Some(());
    }
    draw(game, you, 1);
    if cx.source_on_battlefield(game) {
        untap_permanent(game, cx.source_id());
        remove_from_combat(game, cx.source_id());
    }
    Some(())
}
