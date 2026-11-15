//! The Binding of the Titans: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after
//! III.) I — Each player mills three cards. II — Exile up to two target cards from graveyards. For each creature
//! card exiled this way, you gain 1 life. III — Return target creature or land card from your graveyard to your
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheBindingOfTheTitans.TheBindingOfTheTitansEffect", apply)];

/// The targets exiled together; 1 life for each creature card of them now in exile
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards = cx.objs(0);
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let life = cards
        .iter()
        .filter(|c| game.obj(**c).zone == Zone::Exile && game.has_type(**c, CardType::Creature))
        .count() as i32;
    if life > 0 {
        gain_life(game, you, life);
    }
    Some(())
}
