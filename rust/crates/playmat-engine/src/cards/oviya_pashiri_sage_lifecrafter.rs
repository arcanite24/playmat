//! Oviya Pashiri, Sage Lifecrafter: "{4}{G}, {T}: Create an X/X colorless Construct artifact creature token, where X
//! is the number of creatures you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("OviyaPashiriSageLifecrafter.OviyaPashiriSageLifecrafterEffect", apply)
        .effects(&["new CreateTokenEffect(new OviyaPashiriSageLifecrafterToken())"])];

/// `OviyaPashiriSageLifecrafterToken(creatures)`: the creatures you control counted as it resolves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .count() as i32;
    let mut e = cx.effects.first()?.clone();
    if let EffectDef::CreateTokenEffect { token, .. } = &mut e {
        token.power = creatures;
        token.toughness = creatures;
    }
    cx.run(game, d, &[e]);
    Some(())
}
