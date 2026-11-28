//! Speakeasy Server: "Flying. When Speakeasy Server enters the battlefield, you gain 1 life for each other creature you
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpeakeasyServer.SpeakeasyServerEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let n = game
        .battlefield
        .iter()
        .filter(|o| **o != src && game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .count() as i32;
    if n > 0 {
        gain_life(game, you, n);
    }
    Some(())
}
