//! Volcanic Vision: "Return target instant or sorcery card from your graveyard to your hand. Volcanic Vision deals
//! damage equal to that card's mana value to each creature your opponents control. Exile Volcanic Vision."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("VolcanicVision.VolcanicVisionReturnToHandTargetEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let you = cx.controller();
    game.move_to(c, Zone::Hand);
    let damage = game.obj(c).def.mana_value() as i32;
    if damage <= 0 {
        return Some(());
    }
    let hit: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| game.has_type(*p, CardType::Creature) && game.obj(*p).controller != you)
        .collect();
    for p in hit {
        deal_damage(game, cx.source_id(), Target::Obj(p), damage, false);
    }
    Some(())
}
