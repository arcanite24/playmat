//! Park Heights Pegasus: "Flying, trample. Whenever Park Heights Pegasus deals combat damage to a player, draw a card
//! if you had two or more creatures enter the battlefield under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ParkHeightsPegasus.ParkHeightsPegasusEffect", apply)
    .effects(&["new DrawCardSourceControllerEffect(1)"])];

/// `PermanentsEnteredBattlefieldWatcher.getThisTurnEnteringPermanents(you)`: two or more creatures, then a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures = game
        .watch
        .entered
        .iter()
        .zip(game.watch.entered_permanents.iter())
        .filter(|(seen, (_, _, controller))| seen.2 && *controller == you)
        .count();
    if creatures >= 2 {
        cx.run(game, d, &cx.effects[..1]);
    }
    (!d.stopped()).then_some(())
}
