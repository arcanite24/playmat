//! Patient Rebuilding: "At the beginning of your upkeep, target opponent mills three cards, then you draw a card for
//! each land card put into that graveyard this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PatientRebuilding.PatientRebuildingEffect", apply)];

/// The lands among the milled cards that reached the graveyard
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let top: Vec<ObjId> = game.player(p).library.iter().take(3).copied().collect();
    note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    let lands = top.iter().filter(|c| game.obj(**c).zone == Zone::Graveyard && game.has_type(**c, CardType::Land)).count();
    if lands > 0 {
        draw(game, cx.controller(), lands);
    }
    Some(())
}
