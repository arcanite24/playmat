//! Solar Blaze: "Each creature deals damage to itself equal to its power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SolarBlaze.SolarBlazeEffect", apply)];

/// each creature in battlefield order, its power as it deals the damage
fn apply(game: &mut Game, _d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    for c in creatures {
        if game.obj(c).zone != Zone::Battlefield {
            continue;
        }
        let (power, _) = game.power_toughness(c);
        deal_damage(game, c, Target::Obj(c), power, false);
    }
    Some(())
}
