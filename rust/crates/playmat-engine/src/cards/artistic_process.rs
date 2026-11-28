//! Artistic Process: "Choose one — … • Create a 3/3 blue and red Elemental creature token with flying. It gains
//! haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ArtisticProcess.ArtisticProcessEffect", apply).tokens(&["Elemental33BlueRedToken"])];

/// the token, then haste until end of turn on it (`FixedTargets(token)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: crate::card::Amount::Fixed(1),
        tapped_and_attacking: false,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: true,
        also: None,
        attach_source: None,
    };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
