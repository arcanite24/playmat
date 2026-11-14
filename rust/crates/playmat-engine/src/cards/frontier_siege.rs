//! Frontier Siege: "Dragons — Whenever a creature with flying you control enters, you may have it fight target
//! creature you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FrontierSiege.FrontierSiegeFightEffect", apply)];

/// The creature that entered (the trigger's target pointer) fights the ability's target, both still creatures on the
/// battlefield (`Permanent.fight`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(a), Some(b)) = (cx.item.entering, cx.obj(0)) else { return Some(()) };
    let creature = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
    if creature(game, a) && creature(game, b) {
        let pb = game.power_toughness(b).0;
        let pa = game.power_toughness(a).0;
        deal_damage(game, b, Target::Obj(a), pb, false);
        let pa = pa.min(fight_lethal(game, b, a));
        deal_damage(game, a, Target::Obj(b), pa, false);
    }
    Some(())
}
