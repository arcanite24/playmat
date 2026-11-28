//! Dusk // Dawn's Dawn: "Return all creature cards with power 2 or less from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DuskDawn.DawnEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Creature) && game.power_toughness(*c).0 < 3)
        .collect();
    for c in cards {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
