//! `CreaturesBecomeOtherTypeEffect(filter, subType, Duration.EndOfTurn)`: "each Elf creature you control becomes a
//! Dinosaur in addition to its other creature types" until end of turn (Allosaurus Shepherd). Layer 4, the filter
//! looked at each time (`getActivePermanents`).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("CreaturesBecomeOtherTypeEffect", affects, Layer::Type(subtype))
        .args(&["filter", "text", "one_of:Duration.EndOfTurn"])];

fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.matches(game, 0, id)
}

/// `SubType.DINOSAUR` as its name: "Dinosaur"
fn subtype(_: &Game, cx: &StaticCx, _: ObjId) -> Vec<String> {
    let Some(arg) = cx.args.get(1) else { return Vec::new() };
    let constant = arg.trim().trim_start_matches("SubType.");
    let name: String = constant
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().chain(c.flat_map(|x| x.to_lowercase())).collect()).unwrap_or_default()
        })
        .collect::<Vec<String>>()
        .join(" ");
    vec![name]
}
