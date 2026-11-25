//! `AddManaFromColorChoicesEffect(ManaType.RED, ManaType.GREEN)`: "add {R} or {G}" (Dal R'El, Augment Orphan's
//! landfall). `produceMana` asks `ManaType.getChoiceOfManaTypes(types, false)`, "Pick a mana type" among the colors
//! in Black, Red, Blue, Green, White order (one choice: no question); only the two-color form is read.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AddManaFromColorChoicesEffect", apply).args(&["text", "text"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let of = |a: &str| match a.trim().rsplit('.').next() {
        Some("WHITE") => Some(Color::W),
        Some("BLUE") => Some(Color::U),
        Some("BLACK") => Some(Color::B),
        Some("RED") => Some(Color::R),
        Some("GREEN") => Some(Color::G),
        _ => None,
    };
    let wanted: Vec<Color> = cx.args.iter().filter_map(|a| of(a)).collect();
    let name = |c: Color| ["White", "Blue", "Black", "Red", "Green"][c.index()];
    let choices: Vec<Color> =
        [Color::B, Color::R, Color::U, Color::G, Color::W].into_iter().filter(|c| wanted.contains(c)).collect();
    let you = cx.controller();
    let chosen = match choices.len() {
        0 => return Some(()),
        1 => choices[0],
        _ => {
            let q = Query::Choice {
                message: "Pick a mana type".into(),
                choices: choices.iter().map(|c| name(*c).to_string()).collect(),
            };
            loop {
                match ask(game, d, you, &q) {
                    Answer::Text(t) => {
                        if let Some(c) = choices.iter().find(|c| name(**c) == t) {
                            break *c;
                        }
                    }
                    Answer::Stop => return None,
                    _ => {}
                }
                if d.stopped() {
                    return None;
                }
            }
        }
    };
    game.player_mut(you).mana_pool.colored[chosen.index()] += 1;
    Some(())
}
