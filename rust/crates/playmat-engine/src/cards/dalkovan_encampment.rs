//! Dalkovan Encampment: "{2}{W}, {T}: Whenever you attack this turn, create two 1/1 red Warrior creature tokens that
//! are tapped and attacking. Sacrifice them at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DalkovanEncampment.DalkovanEncampmentEffect", apply)
    .tokens(&["RedWarriorToken"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(token) = cx.tokens.first() else { return Some(()) };
    let e = EffectDef::CreateTokenEffect {
        token: token.clone(),
        amount: crate::card::Amount::Fixed(2),
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
    (!d.stopped()).then_some(())
}
