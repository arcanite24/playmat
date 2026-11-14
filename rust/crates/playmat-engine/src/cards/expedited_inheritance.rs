//! Expedited Inheritance: "Whenever a creature is dealt damage, its controller may exile that many cards from the
//! top of their library. They may play those cards until the end of their next turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExpeditedInheritance.ExpeditedInheritanceExileEffect", apply)
    .effects(&["new ExileTopXMayPlayUntilEffect(1, Duration.UntilEndOfYourNextTurn)"])];

/// The creature's controller (the trigger's pointer) is asked, then exiles X (the damage) and may play them
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.item.x;
    let Some(p) = cx.player(0) else { return Some(()) };
    if n <= 0 {
        return Some(());
    }
    let message = if n > 1 {
        format!(
            "Exile {} cards from the top of your library. Until the end of your next turn, you may play those cards.",
            crate::log::number(n as usize)
        )
    } else {
        "Exile the top card of your library. Until the end of your next turn, you may play that card.".to_string()
    };
    if !choose_use(game, d, p, &message)? {
        return Some(());
    }
    let mut e = cx.effects[0].clone();
    if let EffectDef::ExileTopXMayPlayUntilEffect { amount, .. } = &mut e {
        *amount = crate::card::Amount::Fixed(n);
    }
    cx.run_as(game, d, &[e], p);
    Some(())
}
