//! Lorehold Excavation: "At the beginning of your end step, mill a card. If a land card was milled this way, you
//! gain 1 life. Otherwise, Lorehold Excavation deals 1 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LoreholdExcavation.LoreholdExcavationEffect", apply)
    .effects(&["new DamagePlayersEffect(1, TargetController.OPPONENT)"])];

/// The milled card (in the graveyard) a land: 1 life; else the damage
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top = game.player(you).library.first().copied();
    cx.run(game, d, &[EffectDef::MillCardsControllerEffect { amount: Amount::Fixed(1) }]);
    if d.stopped() {
        return None;
    }
    if top.is_some_and(|c| game.obj(c).zone == Zone::Graveyard && game.obj(c).is(CardType::Land)) {
        gain_life(game, you, 1);
        return Some(());
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
