//! Tazeem Raptor: "When Tazeem Raptor enters the battlefield, you may return a land you control to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TazeemRaptor.TazeemRaptorEffect", apply)];

/// `TargetControlledPermanent(0, 1, land, notTarget)` chosen with `Player.choose` (asked even with nothing to
/// choose), the land chosen to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Land))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?;
    if let Some(&o) = chosen.first() {
        if game.obj(o).zone == Zone::Battlefield {
            game.move_to(o, Zone::Hand);
        }
    }
    Some(())
}
