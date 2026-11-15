//! Geier Reach Bandit // Vildin-Pack Alpha: "Whenever a Werewolf you control enters, you may transform it." (the
//! trigger's pointer is the Werewolf that entered; `Permanent.transform` while it's still on the battlefield)

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("GeierReachBandit.VildinPackAlphaEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(o) = cx.obj(0) {
        if game.obj(o).zone == Zone::Battlefield {
            transform(game, o);
        }
    }
    Some(())
}
