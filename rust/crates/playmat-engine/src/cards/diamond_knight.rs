//! Diamond Knight: "As Diamond Knight enters, choose a color. Whenever you cast a spell of the chosen color, put a
//! +1/+1 counter on Diamond Knight."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("DiamondKnight.DiamondKnightPredicate", of_chosen_color)];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

/// the spell shares the color chosen as the Knight entered (`ChooseColorEffect`'s "Chosen color: …" line)
fn of_chosen_color(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(source) = source else { return false };
    let Some((_, line)) = game.obj(source).info.iter().find(|(k, _)| k == "chosen color") else { return false };
    let Some(k) = NAMES.iter().position(|n| line.contains(&format!("Chosen color: {n}<"))) else { return false };
    game.colors(id).contains(&crate::card::Color::ALL[k])
}
