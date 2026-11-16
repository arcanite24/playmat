//! Hall of Triumph: "As Hall of Triumph enters, choose a color. Creatures you control of the chosen color get
//! +1/+1."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "HallOfTriumph.HallOfTriumphBoostControlledEffect",
    of_chosen_color,
    Layer::ModifyPt(plus_one),
)];

/// the color its `ChooseColorEffect` chose (`getValue(sourceId + "_color")`), from the source's info line
/// (`<font color = 'blue'>Chosen color: Red</font>`)
pub(super) fn chosen_color(game: &Game, source: ObjId) -> Option<Color> {
    let (_, text) = game.obj(source).info.iter().find(|(k, _)| k == "chosen color")?;
    let names = ["White", "Blue", "Black", "Red", "Green"];
    let k = names.iter().position(|n| text.contains(&format!("Chosen color: {n}<")))?;
    Some(Color::ALL[k])
}

/// a creature you control that has the chosen color
fn of_chosen_color(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let Some(c) = chosen_color(game, cx.source) else { return false };
    let o = game.obj(id);
    o.zone == Zone::Battlefield
        && o.controller == cx.controller
        && game.has_type(id, CardType::Creature)
        && game.colors(id).contains(&c)
}

fn plus_one(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (1, 1)
}
