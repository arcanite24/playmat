//! `GainControlAllOwnedEffect(filter)`: "each player gains control of all creatures they own" (Trostani
//! Discordant). For the rest of the game (layer 2, `Duration.EndOfGame`): the newest control change for each of them.

use super::*;
use crate::state::{ControlChange, ControlUntil, Zone};

pub const HANDLERS: &[Handler] = &[Handler::effect("GainControlAllOwnedEffect", apply).args(&["filter"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first()?;
    let objs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).zone == Zone::Battlefield && game.filter_matches(filter, you, Some(cx.source_id()), *o))
        .collect();
    for o in objs {
        let (owner, before, zcc) = {
            let p = game.obj(o);
            (p.owner, p.controller, p.zcc)
        };
        game.control_changes.push(ControlChange { target: o, zcc, to: owner, before, until: ControlUntil::Forever });
        if before != owner {
            set_controller(game, o, owner);
        }
    }
    refresh_control(game);
    Some(())
}
