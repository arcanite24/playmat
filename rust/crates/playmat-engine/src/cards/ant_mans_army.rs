//! Ant-Man's Army: "When this creature enters, create a Food token or a Treasure token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("AntMansArmy.AntMansArmyEffect", apply).tokens(&["FoodToken", "TreasureToken"])];

/// `chooseUse("Create a Food token or a Treasure token?", "Food", "Treasure")`: yes is Food
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let food = choose_use(game, d, cx.controller(), "Create a Food token or a Treasure token?")?;
    let token = if food { &cx.tokens[0] } else { &cx.tokens[1] };
    cx.create_tokens(game, d, token, 1);
    (!d.stopped()).then_some(())
}
