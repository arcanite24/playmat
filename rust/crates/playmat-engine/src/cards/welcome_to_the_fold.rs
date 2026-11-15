//! Welcome to the Fold: "Madness {X}{U}{U}. Gain control of target creature if its toughness is 2 or less. If this
//! spell's madness cost was paid, instead gain control of that creature if its toughness is X or less."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WelcomeToTheFold.WelcomeToTheFoldEffect", apply)
    .args(&["text", "text"])
    .effects(&["new GainControlTargetEffect(Duration.Custom, true)"])];

/// `init`: the most toughness is 2, or X when the spell was cast for a cost with {X} (madness casts the card with
/// that cost as its own); a target with more isn't taken (`discard()`), else control for good
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let most = if game.obj(cx.source_id()).def.cost.x > 0 { cx.item.x } else { 2 };
    if game.obj(o).zone == Zone::Battlefield && game.power_toughness(o).1 > most {
        return Some(());
    }
    cx.run(game, d, cx.effects);
    Some(())
}
