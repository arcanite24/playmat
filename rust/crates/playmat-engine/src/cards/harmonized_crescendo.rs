//! Harmonized Crescendo: "Choose a creature type. Draw a card for each permanent you control of that type."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HarmonizedCrescendo.HarmonizedCrescendoEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let q = Query::Choice { message: "Choose a creature type".into(), choices: Vec::new() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
    };
    if log::kept(game) {
        let line = format!("{}: {} chooses {}", log::object(game, cx.source_id()), log::player(you), chosen);
        log::inform(game, line);
    }
    let n = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == you && game.has_subtype(**b, &chosen))
        .count();
    if n > 0 {
        draw(game, you, n);
    }
    Some(())
}
