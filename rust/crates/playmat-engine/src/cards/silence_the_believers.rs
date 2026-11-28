//! Silence the Believers: "Strive — This spell costs {2}{B} more to cast for each target beyond the first. Exile any
//! number of target creatures and all Auras attached to them."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SilenceTheBelievers.SilenceTheBelieversExileEffect", apply)];

/// each target in order: its Auras (in attachment order), then the creature, exiled one by one
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for creature in cx.objs(0) {
        if game.obj(creature).zone != Zone::Battlefield {
            continue;
        }
        let auras: Vec<ObjId> = game.attachments(creature).filter(|a| game.has_subtype(*a, "Aura")).collect();
        for a in auras {
            if game.obj(a).zone == Zone::Battlefield {
                game.move_to(a, Zone::Exile);
            }
        }
        if game.obj(creature).zone == Zone::Battlefield {
            game.move_to(creature, Zone::Exile);
        }
    }
    Some(())
}
