//! Carrion Locust: "When Carrion Locust enters, exile target card from an opponent's graveyard. If it was a creature
//! card, that player loses 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CarrionLocust.CarrionLocustEffect", apply)];

/// whether it was a creature card, and its owner, read before the exile
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let c = cx.obj(0)?;
    let creature = game.obj(c).is(CardType::Creature);
    let owner = game.obj(c).owner;
    game.move_to(c, Zone::Exile);
    if creature && !game.player(owner).lost {
        lose_life(game, owner, 1);
    }
    Some(())
}
