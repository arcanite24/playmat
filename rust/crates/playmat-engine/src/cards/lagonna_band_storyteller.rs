//! Lagonna-Band Storyteller: "you may put target enchantment card from your graveyard on top of your library. If you
//! do, you gain life equal to its mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LagonnaBandStoryteller.LagonnaBandStorytellerEffect", apply)];

/// The enchantment, still in the graveyard, on top; then its mana value in life
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    if game.obj(card).zone != Zone::Graveyard || !game.obj(card).def.is(CardType::Enchantment) {
        return None;
    }
    let mv = game.obj(card).def.mana_value() as i32;
    to_library(game, card, true);
    gain_life(game, cx.controller(), mv);
    Some(())
}
