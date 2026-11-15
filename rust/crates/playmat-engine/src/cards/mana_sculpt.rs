//! Mana Sculpt: "Counter target spell. If you control a Wizard, add an amount of {C} equal to the amount of mana
//! spent to cast that spell at the beginning of your next main phase."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ManaSculpt.ManaSculptEffect", apply).effects(&["new CounterTargetEffect()"])];

/// the mana spent on it (`ManaPaidSourceWatcher.getTotalPaid`), read before it's countered; the delayed trigger is
/// made whether or not it was countered
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0)?;
    let spent = game.obj(spell).mana_spent;
    cx.run(game, d, cx.effects);
    let you = cx.controller();
    let wizard = game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Wizard"));
    if wizard {
        let add = EffectDef::BasicManaEffect { colored: [0; 5], colorless: spent, until_end_of_turn: false };
        let rule = format!("At the beginning of your next main phase, add {}.", "{C}".repeat(spent as usize));
        crate::rules::next_main_delayed(game, cx.source, you, vec![add], rule, "main");
    }
    Some(())
}
