//! Containment Breach: "Destroy target artifact or enchantment. If its mana value is 2 or less, create a 1/1 black
//! and green Pest creature token with 'When this creature dies, you gain 1 life.'"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ContainmentBreach.ContainmentBreachEffect", apply)
    .tokens(&["PestBlackGreenDiesToken"])];

/// The token whether or not the destroy worked, as in Java
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let mana_value = game.obj(o).def.mana_value();
    cx.run_on(game, d, &[EffectDef::DestroyTargetEffect], vec![Target::Obj(o)]);
    if mana_value <= 2 {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    Some(())
}
