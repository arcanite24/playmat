//! `AttachToSourceTargetEffect`: "attach target Equipment you control to {this}" (Kazuul's Toll Collector): each
//! target still there is attached to the source, if the source is still on the battlefield
//! (`Permanent.addAttachment`).

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::effect("AttachToSourceTargetEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let to = cx.source_id();
    for a in cx.objs(0) {
        // an Equipment that's a creature doesn't attach (301.5c)
        if game.obj(a).zone == Zone::Battlefield && a != to && !game.has_type(a, CardType::Creature) {
            let z = game.obj(to).zcc;
            game.attach_to(a, to, z);
        }
    }
    Some(())
}
