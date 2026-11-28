//! Duneblast: "Choose up to one creature. Destroy the rest."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Duneblast.DuneblastEffect", apply)];

/// `TargetCreaturePermanent(0, 1).withNotTarget(true)`: up to one creature to keep, then destroy every other
/// creature, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    let mut keep = None;
    if !creatures.is_empty() {
        loop {
            match ask(game, d, you, &Query::Cards { objs: creatures.clone(), required: false }) {
                Answer::Obj(o) if creatures.contains(&o) => {
                    keep = Some(o);
                    break;
                }
                Answer::Stop => return None,
                Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                _ => break,
            }
            if d.stopped() {
                return None;
            }
        }
    }
    for o in creatures {
        if Some(o) != keep && game.obj(o).zone == Zone::Battlefield {
            destroy(game, o);
        }
    }
    Some(())
}
