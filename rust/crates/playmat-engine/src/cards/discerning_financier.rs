//! Discerning Financier: "At the beginning of your upkeep, if an opponent controls more lands than you, create a
//! Treasure token. {2}{W}: Choose another player. That player gains control of target Treasure you control. You draw
//! a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiscerningFinancier.DiscerningFinancierEffect", apply)];

/// With the Treasure still there: another player (`TargetPlayer(NOT_YOU)`, not targeted) chosen, who controls it
/// from then on (`GainControlTargetEffect(Duration.WhileOnBattlefield, true, player)` on a `FixedTarget`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_target(game, d, you, &crate::card::TargetDef::TargetOpponent, Some(cx.source_id()), &rule, false);
    if d.stopped() {
        return None;
    }
    let Some(Target::Player(p)) = chosen.and_then(|c| c.first().copied()) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let before = game.obj(o).controller;
    let zcc = game.obj(o).zcc;
    game.control_changes.push(crate::state::ControlChange {
        target: o,
        zcc,
        to: p,
        before,
        until: crate::state::ControlUntil::Forever,
    });
    if before != p {
        set_controller(game, o, p);
    }
    refresh_control(game);
    Some(())
}
