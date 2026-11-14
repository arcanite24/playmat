//! Bruse Tarl, Roving Rancher: "Whenever Bruse Tarl enters or attacks, exile the top card of your library. If it's a
//! land card, create a 2/2 white Ox creature token. Otherwise, you may cast it until the end of your next turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BruseTarlRovingRancher.BruseTarlRovingRancherEffect", apply).tokens(&["Ox22Token"])];

/// A land is exiled and makes an Ox; anything else is exiled playable until the end of your next turn (being
/// nonland, it can only be cast)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    if game.obj(card).is(CardType::Land) {
        game.move_to(card, Zone::Exile);
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    } else {
        let e = EffectDef::ExileTopXMayPlayUntilEffect {
            amount: Amount::Fixed(1),
            choose_one: false,
            duration: "UntilEndOfYourNextTurn".into(),
        };
        cx.run(game, d, &[e]);
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
