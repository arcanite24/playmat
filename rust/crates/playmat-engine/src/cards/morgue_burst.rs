//! Morgue Burst: "Return target creature card from your graveyard to your hand. Morgue Burst deals damage to any
//! target equal to the power of the card returned this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MorgueBurst.MorgueBurstEffect", apply)];

/// The card to its owner's hand, then its printed power as damage to the second target
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    game.move_to(card, Zone::Hand);
    let damage = game.obj(card).def.power;
    match cx.target(1)? {
        Target::Obj(o) if game.obj(o).zone == Zone::Battlefield => {
            deal_damage(game, cx.source_id(), Target::Obj(o), damage, false);
        }
        Target::Player(p) => {
            deal_damage(game, cx.source_id(), Target::Player(p), damage, false);
        }
        Target::Obj(_) => return None,
    }
    Some(())
}
