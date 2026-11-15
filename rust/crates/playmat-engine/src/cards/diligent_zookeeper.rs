//! Diligent Zookeeper: "Each non-Human creature you control gets +1/+1 for each of its creature types, to a maximum
//! of 10."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("DiligentZookeeper.DiligentZookeeperEffect", affects, Layer::ModifyPt(per_type))];

/// non-Human creatures you control
fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.obj(id).controller == cx.controller
        && game.has_type(id, CardType::Creature)
        && !game.has_subtype(id, "Human")
}

/// +10/+10 with every creature type (`isAllCreatureTypes`), else +1/+1 for each creature type it has, at most 10:
/// its own and those given to it, each counted once
fn per_type(game: &Game, _: &StaticCx, id: ObjId) -> (i32, i32) {
    let o = game.obj(id);
    if o.def.all_creature_types && o.creature_types.is_none() {
        return (10, 10);
    }
    let mut seen: Vec<&str> = Vec::new();
    let candidates = o
        .def
        .subtypes
        .iter()
        .map(|s| s.as_str())
        .chain(o.creature_types.iter().flatten().map(|s| s.as_str()))
        .chain(game.added_subtypes(id));
    for s in candidates {
        if !crate::card::NON_CREATURE_TYPES.contains(&s) && !seen.contains(&s) && game.has_subtype(id, s) {
            seen.push(s);
        }
    }
    let n = seen.len().min(10) as i32;
    (n, n)
}
