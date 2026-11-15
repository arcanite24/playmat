//! Immortal Servitude: "Return each creature card with mana value X from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ImmortalServitude.ImmortalServitudeEffect", apply)];

/// X as cast; the creature cards of that mana value move to the battlefield together (`moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0) as u32;
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.has_type(*c, CardType::Creature) && game.obj(*c).def.mana_value() == x)
        .collect();
    if !cards.is_empty() && !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
