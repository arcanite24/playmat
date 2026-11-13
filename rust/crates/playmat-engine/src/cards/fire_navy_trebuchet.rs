//! Fire Navy Trebuchet: "Whenever you attack, create a 2/1 colorless Construct artifact creature token with flying
//! named Ballistic Boulder that's tapped and attacking. Sacrifice that token at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("FireNavyTrebuchet.FireNavyTrebuchetEffect", apply).tokens(&["BallisticBoulderToken"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: crate::card::Amount::Fixed(1),
        tapped_and_attacking: true,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: true,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run(game, d, &[e]);
    Some(())
}
