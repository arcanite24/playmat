//! Seed Guardian: "When Seed Guardian dies, create an X/X green Elemental creature token, where X is the number of
//! creature cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SeedGuardian.SeedGuardianEffect", apply).tokens(&["ElementalXXGreenToken"])];

/// `new ElementalXXGreenToken(creaturesInGraveyard)`, counted as it resolves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.player(you).graveyard.iter().filter(|c| game.obj(**c).is(CardType::Creature)).count() as i32;
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: crate::card::Amount::Fixed(n) };
    cx.run_on(game, d, &[e], Vec::new());
    (!d.stopped()).then_some(())
}
