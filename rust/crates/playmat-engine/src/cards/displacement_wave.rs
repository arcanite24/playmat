//! Displacement Wave: "Return all nonland permanents with mana value X or less to their owners' hands."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DisplacementWave.DisplacementWaveEffect", apply)];

/// Every nonland permanent with mana value X or less goes to its owner's hand at once (`moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    let cards: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| !game.has_type(*o, CardType::Land) && game.obj(*o).def.mana_value() as i32 <= x)
        .collect();
    if !cards.is_empty() {
        put_cards(game, d, you, &cards, crate::card::PutCards::Hand);
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
