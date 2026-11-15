//! Lazav, Wearer of Faces: "Whenever Lazav attacks, exile target card from a graveyard, then investigate. Whenever
//! you sacrifice a Clue, you may have Lazav become a copy of a creature card exiled with it until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LazavWearerOfFaces.LazavWearerOfFacesEffect", apply)];

/// Lazav on the battlefield: "Have {this} become a copy of a creature card exiled with it" (as Java words it), then
/// a creature card of its exile zone (`getStackMomentSourceZCC`) chosen and copied until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    if !choose_use(game, d, you, "Have {this} become a copy of a creature card exiled with it")? {
        return Some(());
    }
    let (src, zcc) = cx.source;
    let mut creatures: Vec<ObjId> = Vec::new();
    for (s, z, c) in &game.exiled_for_source {
        let o = game.obj(*c);
        if *s == src && *z == zcc && o.zone == Zone::Exile && o.def.is(CardType::Creature) && !creatures.contains(c) {
            creatures.push(*c);
        }
    }
    if creatures.is_empty() {
        return Some(());
    }
    let Some(&of) = choose_cards(game, d, you, &creatures, &creatures, 1, 1, "creature card to copy")?.first() else {
        return Some(());
    };
    let copy = game.obj(of).copiable();
    become_copy_until_end_of_turn(game, cx.source_id(), copy);
    Some(())
}
