//! Burn Down the House: "Choose one — • Burn Down the House deals 5 damage to each creature and each planeswalker.
//! • Create three 1/1 red Devil creature tokens with "When this creature dies, it deals 1 damage to any target." They
//! gain haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BurnDownTheHouse.BurnDownTheHouseEffect", apply).tokens(&["DevilToken"])];

/// `token.putOntoBattlefield(3, …)`, then haste for those tokens until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: Amount::Fixed(3),
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
    if d.stopped() {
        return None;
    }
    Some(())
}
