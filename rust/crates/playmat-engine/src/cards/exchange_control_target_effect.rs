//! `ExchangeControlTargetEffect(Duration.EndOfGame, text[, withSource])`: "exchange control of two target creatures"
//! (Switcheroo, Shifting Grift) or, with the source, "of this and target nonland permanent" (Eyes Everywhere). Both
//! still on the battlefield and under different controllers, each comes under the other's controller for the rest of
//! the game (layer 2, the newest control change for each).

use super::*;
use crate::state::{ControlChange, ControlUntil, Zone};

pub const HANDLERS: &[Handler] = &[Handler::effect("ExchangeControlTargetEffect", apply).args(&[
    "one_of:Duration.EndOfGame",
    "text",
    "one_of:true|false",
])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let with_source = cx.args.get(2).is_some_and(|a| a.trim() == "true");
    let (first, second) = if with_source {
        if !cx.source_on_battlefield(game) {
            return Some(());
        }
        (cx.obj(0)?, cx.source_id())
    } else {
        let objs = cx.objs(0);
        (*objs.first()?, *objs.get(1)?)
    };
    if game.obj(first).zone != Zone::Battlefield || game.obj(second).zone != Zone::Battlefield {
        return Some(());
    }
    let (a, b) = (game.obj(first).controller, game.obj(second).controller);
    if a == b {
        return Some(());
    }
    for (o, to, before) in [(first, b, a), (second, a, b)] {
        let zcc = game.obj(o).zcc;
        game.control_changes.push(ControlChange { target: o, zcc, to, before, until: ControlUntil::Forever });
        set_controller(game, o, to);
    }
    refresh_control(game);
    Some(())
}
