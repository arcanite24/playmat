//! Liliana's Contract: "At the beginning of your upkeep, if you control four or more Demons with different names, you
//! win the game."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("LilianasContract.LilianasContractCondition", four_demon_names)];

/// `DifferentlyNamedPermanentCount` of the Demons you control (a nameless face-down one counts once)
fn four_demon_names(game: &Game, you: Seat, _: ObjId) -> bool {
    let mut names: Vec<String> = Vec::new();
    let mut nameless = false;
    for o in game.battlefield.iter() {
        if game.obj(*o).controller != you || !game.has_subtype(*o, "Demon") {
            continue;
        }
        let name = game.obj(*o).name().to_string();
        if name.is_empty() {
            nameless = true;
        } else if !names.contains(&name) {
            names.push(name);
        }
    }
    names.len() + usize::from(nameless) >= 4
}
