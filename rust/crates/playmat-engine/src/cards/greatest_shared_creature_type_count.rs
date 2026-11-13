//! `GreatestSharedCreatureTypeCount` (Skemfar Shadowsage): "the greatest number of creatures you control that have
//! a creature type in common"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GreatestSharedCreatureTypeCount", value)];

/// The creatures you control with every creature type (changelings) count for any type; among the rest, the
/// greatest number that share one creature type is added
fn value(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    let all_types = |o: ObjId| game.obj(o).def.all_creature_types && game.obj(o).creature_types.is_none();
    let changelings = creatures.iter().filter(|o| all_types(**o)).count() as i32;
    let rest: Vec<ObjId> = creatures.iter().copied().filter(|o| !all_types(*o)).collect();
    let mut types: Vec<String> = Vec::new();
    for o in &rest {
        let obj = game.obj(*o);
        let own = obj.creature_types.clone().unwrap_or_else(|| obj.def.subtypes.clone());
        for t in own.iter().map(String::as_str).chain(game.added_subtypes(*o)) {
            if !crate::card::NON_CREATURE_TYPES.contains(&t) && !types.iter().any(|x| x == t) {
                types.push(t.to_string());
            }
        }
    }
    let best = types.iter().map(|t| rest.iter().filter(|o| game.has_subtype(**o, t)).count()).max().unwrap_or(0);
    changelings + best as i32
}
