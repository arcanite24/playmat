//! Liliana, Untouched by Death: "+1: Mill three cards. If at least one Zombie card is milled this way, each opponent
//! loses 2 life and you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilianaUntouchedByDeath.LilianaUntouchedByDeathEffect", apply)
    .effects(&["new MillCardsControllerEffect(3)", "new LoseLifeOpponentsEffect(2)", "new GainLifeEffect(2)"])];

/// `millCards(3)`; with a Zombie card among those milled, `LoseLifeOpponentsEffect(2)` and `gainLife(2)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    cx.run(game, d, &cx.effects[..1]);
    let zombie = top
        .iter()
        .any(|c| game.obj(*c).zone == Zone::Graveyard && game.obj(*c).def.subtypes.iter().any(|s| s == "Zombie"));
    if zombie {
        cx.run(game, d, &cx.effects[1..3]);
    }
    Some(())
}
