//! Gorging Vulture: "When Gorging Vulture enters the battlefield, put the top four cards of your library into your
//! graveyard. You gain 1 life for each creature card put into your graveyard this way."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GorgingVulture.GorgingVultureEffect", apply).effects(&["new MillCardsControllerEffect(4)"])
];

/// The mill, then 1 life for each milled creature card that's in the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(4).copied().collect();
    cx.run(game, d, &cx.effects[..1]);
    let n = top.iter().filter(|c| game.obj(**c).zone == Zone::Graveyard && game.obj(**c).is(CardType::Creature)).count();
    if n > 0 {
        gain_life(game, you, n as i32);
    }
    Some(())
}
