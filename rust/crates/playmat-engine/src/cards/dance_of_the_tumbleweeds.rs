//! Dance of the Tumbleweeds: "+ {3} — Create an X/X green Elemental creature token, where X is the number of lands
//! you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DanceOfTheTumbleweeds.DanceOfTheTumbleweedsEffect", apply)
    .effects(&["new CreateTokenEffect(new ElementalXXGreenToken())"])];

/// `LandsYouControlCount` as it resolves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land)).count()
        as i32;
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { token, .. } = &mut e {
        token.power = x;
        token.toughness = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}
