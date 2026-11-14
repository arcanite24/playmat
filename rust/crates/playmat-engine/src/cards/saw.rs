//! Saw: "Whenever equipped creature attacks, you may sacrifice a permanent other than that creature or Saw. If you
//! do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Saw.SawEffect", apply)];

/// `TargetSacrifice(0, 1, another permanent, not the attacking creature)`: one you control, or none
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let attacker = cx.obj(0);
    let can: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && *o != cx.source_id() && Some(*o) != attacker)
        .collect();
    if can.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &can, &can, 0, 1, &rule)?;
    if let Some(o) = chosen.first().copied() {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
            draw(game, you, 1);
        }
    }
    Some(())
}
