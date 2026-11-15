//! Endless Sands: "{T}: Add {C}. {2}, {T}: Exile target creature you control. {4}, {T}, Sacrifice Endless Sands:
//! Return each creature card exiled with Endless Sands to the battlefield under its owner's control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EndlessSands.EndlessSandsEffect", apply)];

/// Every card still in the source's exile zone (`getExileZoneId(game, source)`: the zone-change count the ability
/// was put on the stack with, the permanent's), in the order they were exiled, returns together, each owner's under
/// their control (`moveCards(…, byOwner)`, as Detention Sphere's)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
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
