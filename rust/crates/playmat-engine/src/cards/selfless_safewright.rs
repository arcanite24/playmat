//! Selfless Safewright: "Flash. Convoke. When Selfless Safewright enters, choose a creature type. Other permanents
//! you control of that type gain hexproof and indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SelflessSafewright.SelflessSafewrightEffect", apply)];

/// `ChoiceCreatureType` (told to everyone), then the other permanents you control of it gain hexproof, then
/// indestructible, until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let q = Query::Choice { message: "Choose a creature type".into(), choices: Vec::new() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if log::kept(game) {
        let line = format!("{} chooses {}", log::player(you), chosen);
        log::inform(game, line);
    }
    let permanents: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != cx.source_id() && game.obj(*o).controller == you && game.has_subtype(*o, &chosen))
        .collect();
    for o in &permanents {
        grant(game, *o, &AbilityDef::HexproofAbility);
    }
    for o in &permanents {
        grant(game, *o, &AbilityDef::IndestructibleAbility);
    }
    Some(())
}
