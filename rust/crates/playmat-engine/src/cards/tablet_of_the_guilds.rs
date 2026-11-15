//! Tablet of the Guilds: "As Tablet of the Guilds enters, choose two colors. Whenever you cast a spell, if it's at
//! least one of the chosen colors, you gain 1 life for each of the chosen colors it is."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TabletOfTheGuilds.TabletOfTheGuildsEntersBattlefieldEffect", choose_colors),
    Handler::effect("TabletOfTheGuilds.TabletOfTheGuildsGainLifeEffect", gain),
];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

/// "Choose the first color", then "Choose the second color" among the other four (`ChoiceColor`), kept as the
/// permanent's marks
fn choose_colors(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.source_id();
    let you = cx.controller();
    let mut left: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    let mut chosen: Vec<String> = Vec::new();
    for message in ["Choose the first color", "Choose the second color"] {
        let q = Query::Choice { message: message.into(), choices: left.clone() };
        let c = loop {
            match ask(game, d, you, &q) {
                Answer::Text(t) if left.contains(&t) => break t,
                Answer::Stop => return None,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        };
        left.retain(|n| *n != c);
        chosen.push(c);
    }
    let o = game.obj_mut(id);
    o.marks.retain(|m| !m.starts_with("tablet color "));
    for c in chosen {
        o.marks.push(format!("tablet color {c}"));
    }
    Some(())
}

/// the colors chosen as `id` entered
fn chosen_colors(game: &Game, id: ObjId) -> Vec<Color> {
    game.obj(id)
        .marks
        .iter()
        .filter_map(|m| m.strip_prefix("tablet color "))
        .filter_map(|m| NAMES.iter().position(|n| *n == m).map(|k| Color::ALL[k]))
        .collect()
}

/// The spell still on the stack: 1 life for each chosen color it is
fn gain(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    if game.obj(spell).zone != Zone::Stack {
        return Some(());
    }
    let colors = game.colors(spell);
    let n = chosen_colors(game, cx.source_id()).iter().filter(|c| colors.contains(c)).count() as i32;
    if n > 0 {
        gain_life(game, cx.controller(), n);
    }
    Some(())
}
