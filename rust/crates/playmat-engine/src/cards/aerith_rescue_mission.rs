//! Aerith Rescue Mission: "Choose one — • Take the Elevator — Create three 1/1 colorless Hero creature tokens.
//! • Take 59 Flights of Stairs — Tap up to three target creatures. Put a stun counter on one of them."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AerithRescueMission.AerithRescueMissionStunEffect", apply)];

/// "creature to put a stun counter on": one of the targets still on the battlefield, chosen (not targeted,
/// `TargetPermanent.choose`), gets a stun counter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let them: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    if them.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &them, &them, 1, 1, &rule)?;
    let Some(&o) = chosen.first() else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield {
        put_counters(game, o, "stun", 1, you);
    }
    Some(())
}
