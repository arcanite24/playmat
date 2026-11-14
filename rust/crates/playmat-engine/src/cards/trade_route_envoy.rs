//! Trade Route Envoy: "When Trade Route Envoy enters, draw a card if you control a creature with a counter on it. If
//! you don't draw a card this way, put a +1/+1 counter on Trade Route Envoy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TradeRouteEnvoy.TradeRouteEnvoyEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let countered = game.battlefield.iter().any(|o| {
        game.obj(*o).controller == you
            && game.has_type(*o, CardType::Creature)
            && game.obj(*o).counters.iter().any(|(_, n)| *n > 0)
    });
    if countered && !game.player(you).library.is_empty() {
        draw(game, you, 1);
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        add_counters_by(game, cx.source_id(), "+1/+1", 1, Some(you));
    }
    Some(())
}
