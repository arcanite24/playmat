//! Hubris: "Return target creature and all Auras attached to it to their owners' hands."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Hubris.HubrisReturnEffect", apply)];

/// the target's Auras (in attachment order), then the target, to their owners' hands (one `moveCards`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for creature in cx.objs(0) {
        if game.obj(creature).zone != Zone::Battlefield {
            continue;
        }
        let mut cards: Vec<ObjId> = game.attachments(creature).filter(|a| game.has_subtype(*a, "Aura")).collect();
        cards.push(creature);
        for c in cards {
            if game.obj(c).zone == Zone::Battlefield {
                game.move_to(c, Zone::Hand);
            }
        }
    }
    Some(())
}
