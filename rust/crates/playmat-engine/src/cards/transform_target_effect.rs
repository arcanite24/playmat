//! `TransformTargetEffect`: "transform target …" (Attentive Skywarden's Incubator token, Elvish Vatkeeper, Waxing
//! Moon, Seedpod Caretaker): each target still on the battlefield transforms (`Permanent.transform`).

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("TransformTargetEffect", apply).args(&["one_of:"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield {
            transform(game, o);
        }
    }
    Some(())
}
