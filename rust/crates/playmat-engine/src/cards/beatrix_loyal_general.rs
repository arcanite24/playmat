//! Beatrix, Loyal General: "At the beginning of combat on your turn, you may attach any number of Equipment you
//! control to target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BeatrixLoyalGeneral.BeatrixLoyalGeneralEffect", apply)];

/// You choose any number of Equipment you control (`TargetPermanent(0, MAX, filter, true)`; asked with none too), each attached to the
/// target creature if it's still on the battlefield (one already attached to it stays; an Equipment that's a
/// creature can't be attached)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creature = cx.obj(0)?;
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let equipment: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.obj(*o).def.has_subtype("Equipment"))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &equipment, &equipment, 0, usize::MAX, &rule)?;
    let z = game.obj(creature).zcc;
    for e in chosen {
        if game.obj(e).zone == Zone::Battlefield
            && e != creature
            && !game.has_type(e, CardType::Creature)
            && game.attached(e) != Some(creature)
        {
            game.attach_to(e, creature, z);
        }
    }
    Some(())
}
