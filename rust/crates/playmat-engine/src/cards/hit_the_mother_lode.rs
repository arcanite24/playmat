//! Hit the Mother Lode: "Discover 10. If the discovered card's mana value is less than 10, create a number of tapped
//! Treasure tokens equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HitTheMotherLode.HitTheMotherLodeEffect", apply)
    .effects(&["new CreateTokenEffect(new TreasureToken(), 1, true)"])];

/// The discovered card is the first nonland card with mana value 10 or less from the top (none: 0); then 10 minus
/// its mana value tapped Treasures
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hit = game.player(you).library.iter().copied().find(|c| {
        let def = &game.obj(*c).def;
        !def.is(CardType::Land) && def.mana_value() as i32 <= 10
    });
    let value = hit.map(|c| game.obj(c).def.mana_value() as i32).unwrap_or(0);
    if !discover(game, d, you, 10) {
        return None;
    }
    if value >= 10 {
        return None;
    }
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
        *amount = crate::card::Amount::Fixed(10 - value);
    }
    cx.run(game, d, &[e]);
    Some(())
}
