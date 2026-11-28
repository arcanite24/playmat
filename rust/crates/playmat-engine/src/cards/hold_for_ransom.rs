//! Hold for Ransom: "Enchant creature. Enchanted creature can't attack or block and has "{7}: Hold for Ransom's
//! controller sacrifices it and draws a card. Activate only as a sorcery.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HoldForRansom.HoldForRansomSacrificeEffect", apply)];

/// The granted ability's effect, pointed at the Aura that granted it (`afterGain`'s `FixedTarget`): the Hold for
/// Ransom (by name) attached to the creature that has the ability, while it's on the battlefield; its controller
/// sacrifices it and draws a card
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.source_id();
    let Some(aura) = game
        .attachments(creature)
        .find(|a| game.obj(*a).zone == Zone::Battlefield && game.obj(*a).name() == "Hold for Ransom")
    else {
        return Some(());
    };
    let controller = game.obj(aura).controller;
    sacrifice(game, aura);
    draw(game, controller, 1);
    Some(())
}
