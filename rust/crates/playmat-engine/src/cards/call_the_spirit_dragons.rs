//! Call the Spirit Dragons: "At the beginning of your upkeep, for each color, put a +1/+1 counter on a Dragon you
//! control of that color. If you put +1/+1 counters on five Dragons this way, you win the game."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::effect("CallTheSpiritDragons.CallTheSpiritDragonsEffect", apply)];

/// For white, blue, black, red and green in turn: with a Dragon you control of that color, you choose one
/// (`TargetPermanent`, not targeted) and it gets a +1/+1 counter; five different Dragons that got one win the game
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut dragons: Vec<ObjId> = Vec::new();
    for color in [Color::W, Color::U, Color::B, Color::R, Color::G] {
        let of: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| {
                game.obj(*o).controller == you && game.has_subtype(*o, "Dragon") && game.colors(*o).contains(&color)
            })
            .collect();
        if of.is_empty() {
            continue;
        }
        let picked = choose_cards(game, d, you, &of, &of, 1, 1, &item_rule(game, cx.item))?;
        let Some(o) = picked.first().copied() else { continue };
        if add_counters_by(game, o, "+1/+1", 1, Some(you)) > 0 && !dragons.contains(&o) {
            dragons.push(o);
        }
    }
    if dragons.len() >= 5 {
        cx.run(game, d, &[EffectDef::WinGameSourceControllerEffect]);
    }
    Some(())
}
