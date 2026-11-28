//! Fade from History: "Each player who controls an artifact or enchantment creates a 2/2 green Bear creature
//! token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FadeFromHistory.FadeFromHistoryTokenEffect", apply)
    .effects(&["new CreateTokenEffect(new BearToken())"])];

/// The players controlling an artifact or enchantment, from you in turn order, each create a Bear
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> = (0..game.seats())
        .map(|i| (you + i) % game.seats())
        .filter(|p| {
            game.battlefield.iter().any(|o| {
                game.obj(*o).controller == *p
                    && (game.has_type(*o, CardType::Artifact) || game.has_type(*o, CardType::Enchantment))
            })
        })
        .collect();
    for p in players {
        cx.run_as(game, d, cx.effects, p);
    }
    Some(())
}
