//! Artificer's Hex: "Enchant Equipment. At the beginning of your upkeep, if enchanted Equipment is attached to a
//! creature, destroy that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArtificersHex.ArtificersHexEffect", apply)];

/// The Hex on the battlefield, the Equipment it enchants, the creature that one is attached to
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let on = |game: &Game, at: Option<(ObjId, u32)>| {
        at.filter(|(o, zcc)| game.obj(*o).zone == Zone::Battlefield && game.obj(*o).zcc == *zcc).map(|(o, _)| o)
    };
    let hex = cx.source_id();
    if game.obj(hex).zone != Zone::Battlefield {
        return Some(());
    }
    let Some(equipment) = on(game, game.obj(hex).attached_to) else { return Some(()) };
    let Some(creature) = on(game, game.obj(equipment).attached_to) else { return Some(()) };
    if game.has_type(creature, CardType::Creature) {
        destroy(game, creature);
    }
    Some(())
}
