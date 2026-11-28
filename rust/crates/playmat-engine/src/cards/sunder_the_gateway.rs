//! Sunder the Gateway: "… Incubate 2, then transform an Incubator token you control."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("SunderTheGateway.SunderTheGatewayEffect", apply)];

/// an Incubator token you control (`TargetPermanent` not targeted, chosen with `Player.choose`), transformed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.obj(*o).token && game.has_subtype(*o, "Incubator"))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?;
    if let Some(&o) = chosen.first() {
        if game.obj(o).zone == Zone::Battlefield {
            transform(game, o);
        }
    }
    Some(())
}
