//! Kin-Tree Invocation: "Create an X/X black and green Spirit Warrior creature token, where X is the greatest
//! toughness among creatures you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("KinTreeInvocation.KinTreeInvocationCreateTokenEffect", apply).tokens(&["SpiritWarriorToken"])];

/// `GreatestAmongPermanentsValue.TOUGHNESS_CONTROLLED_CREATURES` (0 with none), as the token's power and toughness
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).1)
        .max()
        .unwrap_or(0);
    let mut token = cx.tokens[0].clone();
    token.power = x;
    token.toughness = x;
    cx.create_tokens(game, d, &token, 1);
    (!d.stopped()).then_some(())
}
