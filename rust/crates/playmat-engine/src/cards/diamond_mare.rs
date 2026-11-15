//! Diamond Mare: "As Diamond Mare enters, choose a color. Whenever you cast a spell of the chosen color, you gain 1 life."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DiamondMare.DiamondMareTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new GainLifeEffect(1), false)",
    of_chosen_color,
)];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

/// the color `ChooseColorEffect` chose as the Mare entered (its "Chosen color: X" info line)
fn chosen(game: &Game, id: ObjId) -> Option<Color> {
    let (_, line) = game.obj(id).info.iter().find(|(k, _)| k == "chosen color")?;
    NAMES.iter().position(|n| line.contains(&format!("Chosen color: {n}"))).map(|k| Color::ALL[k])
}

/// the spell cast shares the chosen color
fn of_chosen_color(game: &Game, item: &StackItem) -> bool {
    let (Some(spell), Some(c)) = (item.cast_spell, chosen(game, item.card)) else { return false };
    game.colors(spell).contains(&c)
}
