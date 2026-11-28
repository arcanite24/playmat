//! Kura, the Boundless Sky: "Create an X/X green Spirit creature token, where X is the number of lands you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KuraTheBoundlessSky.KuraTheBoundlessSkyEffect", apply)
    .tokens(&["SpiritGreenXToken"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == you && game.has_type(**b, CardType::Land))
        .count() as i32;
    let mut token = cx.tokens[0].clone();
    token.power = x;
    token.toughness = x;
    cx.create_tokens(game, d, &token, 1);
    Some(())
}
