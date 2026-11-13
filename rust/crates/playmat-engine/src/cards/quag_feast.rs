//! Quag Feast: "Choose target creature, planeswalker, or Vehicle. Mill two cards, then destroy the chosen permanent
//! if its mana value is less than or equal to the number of cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("QuagFeast.QuagFeastEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, &[EffectDef::MillCardsControllerEffect { amount: crate::card::Amount::Fixed(2) }]);
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield
        && game.obj(o).def.mana_value() as usize <= game.player(you).graveyard.len()
    {
        cx.run_on(game, d, &[EffectDef::DestroyTargetEffect], vec![Target::Obj(o)]);
    }
    Some(())
}
