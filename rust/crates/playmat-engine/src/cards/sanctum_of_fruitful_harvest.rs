//! Sanctum of Fruitful Harvest: "At the beginning of your precombat main phase, add X mana of any one color, where X
//! is the number of Shrines you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SanctumOfFruitfulHarvest.DynamicManaEffect", apply)
    .args(&["one_of:Mana.AnyMana(1)", "amount", "text", "one_of:true"])];

/// `DynamicManaEffect.produceMana` with `Mana.AnyMana` and one choice: nothing when X is 0, else a `ChoiceColor`
/// ("Choose color", nothing owed: it resolves outside a payment) and X of that color
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_amount(game, 1)?.max(0) as u32;
    if n == 0 {
        return Some(());
    }
    let you = cx.controller();
    let c = any_color_owed(game, d, you, None, "Choose color")?;
    game.player_mut(you).mana_pool.colored[c.index()] += n;
    Some(())
}
