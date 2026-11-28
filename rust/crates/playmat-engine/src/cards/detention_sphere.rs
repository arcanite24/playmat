//! Detention Sphere: "When Detention Sphere enters, you may exile target nonland permanent not named Detention
//! Sphere and all other permanents with the same name as that permanent. When Detention Sphere leaves the
//! battlefield, return the exiled cards to the battlefield under their owner's control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("DetentionSphere.DetentionSphereEntersEffect", enters),
    Handler::effect("DetentionSphere.DetentionSphereLeavesEffect", leaves),
];

/// The target, or (with a name) every permanent with its name, in battlefield order, exiled into the source's exile
/// zone (`getExileZoneId` with the zone-change count the ability was put on the stack with)
fn enters(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.obj(0) else { return Some(()) };
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let name = game.obj(target).name().to_string();
    let exiled: Vec<ObjId> = if name.is_empty() {
        vec![target]
    } else {
        game.battlefield.iter().copied().filter(|o| game.obj(*o).name() == name).collect()
    };
    let (src, zcc) = cx.source;
    for o in exiled {
        game.move_to(o, Zone::Exile);
        game.exiled_for_source.push((src, zcc, o));
    }
    Some(())
}

/// The cards still in that exile zone (the permanent that left had the zone-change count they were exiled with:
/// one less than the source's now) return together, each owner's under their control
fn leaves(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (src, zcc) = cx.source;
    let exiled: Vec<ObjId> = game
        .exiled_for_source
        .iter()
        .filter(|(s, z, c)| *s == src && (*z == zcc || *z + 1 == zcc) && game.obj(*c).zone == Zone::Exile)
        .map(|(_, _, c)| *c)
        .collect();
    let mut owners: Vec<Seat> = Vec::new();
    for c in &exiled {
        let owner = game.obj(*c).owner;
        if !owners.contains(&owner) {
            owners.push(owner);
        }
    }
    for owner in owners {
        let cards: Vec<ObjId> = exiled.iter().copied().filter(|c| game.obj(*c).owner == owner).collect();
        if !put_cards(game, d, owner, &cards, crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    Some(())
}
