//! `FederationCount`: "the number of creature types among non-Borg creatures you control" (Captain Jonathan Archer)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("FederationCount", count)];

/// the distinct creature types in the subtype lists of the creatures you control that aren't Borg (`getSubtype`:
/// a changeling's list is its own, its "all creature types" isn't in it; `hasSubtype(BORG)` does see it)
fn count(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let mut types: Vec<String> = Vec::new();
    for &o in game.battlefield.iter() {
        let ob = game.obj(o);
        if ob.controller != you || !game.has_type(o, CardType::Creature) || game.has_subtype(o, "Borg") {
            continue;
        }
        let mut listed: Vec<String> = ob.creature_types.clone().unwrap_or_else(|| ob.def.subtypes.clone());
        listed.extend(game.added_subtypes(o).into_iter().map(str::to_string));
        for t in listed {
            if !crate::card::NON_CREATURE_TYPES.contains(&t.as_str()) && game.has_subtype(o, &t) && !types.contains(&t) {
                types.push(t);
            }
        }
    }
    types.len() as i32
}
