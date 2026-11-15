//! Tovolar, Dire Overlord // Tovolar, the Midnight Scourge: "Whenever a Wolf or Werewolf you control deals combat
//! damage to a player, draw a card. At the beginning of your upkeep, if you control three or more Wolves and/or
//! Werewolves, it becomes night. Then transform any number of Human Werewolves you control. Daybound"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TovolarDireOverlord.TovolarDireOverlordEffect", apply)];

/// `game.setDaytime(false)` (`BECOMES_DAY_NIGHT` only when day becomes night), then any number of Human Werewolves
/// you control chosen (not targeted) and each transformed, in the order chosen
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.day == Some(true) {
        game.day = Some(false);
        let sources = game.trigger_sources();
        let fired = game.triggers(sources, |_, _, _, t| matches!(t, Trigger::DayNightChanges));
        game.pending.extend(fired);
    } else {
        game.day = Some(false);
    }
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Human") && game.has_subtype(*o, "Werewolf"))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, possible.len(), "Human Werewolves you control")?;
    for c in chosen {
        let o = game.obj(c);
        if o.zone == Zone::Battlefield && (o.def.transform_back.is_some() || o.front.is_some()) {
            transform(game, c);
        }
    }
    Some(())
}
