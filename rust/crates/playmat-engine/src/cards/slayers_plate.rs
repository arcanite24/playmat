//! Slayer's Plate: "Equipped creature gets +4/+2. Whenever equipped creature dies, if it was a Human, create a 1/1
//! white Spirit creature token with flying. Equip {3}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SlayersPlate.SlayersPlateTriggeredAbility",
    "new DiesAttachedTriggeredAbility(new CreateTokenEffect(new SpiritWhiteToken()), \"equipped creature\")",
    human,
)];

/// `checkTrigger`: the creature that died with the Plate attached (its last known attachments; the one that left
/// last when there are several) was a Human (its subtypes as a card: the engine keeps no last known subtypes)
fn human(game: &Game, item: &StackItem) -> bool {
    let (plate, zcc) = (item.card, game.obj(item.card).zcc);
    (0..game.objs.len() as u32)
        .map(ObjId)
        .filter(|o| {
            let c = game.obj(*o);
            c.zone != Zone::Battlefield && c.lki_attachments.contains(&(plate, zcc))
        })
        .min_by_key(|o| {
            let c = game.obj(*o);
            let yard = &game.player(c.owner).graveyard;
            let from_top = yard.iter().rev().position(|g| g == o).unwrap_or(if c.token { 0 } else { usize::MAX });
            (from_top, std::cmp::Reverse(o.0))
        })
        .is_some_and(|o| game.has_subtype(o, "Human"))
}
