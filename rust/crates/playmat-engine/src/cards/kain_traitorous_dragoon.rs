//! Kain, Traitorous Dragoon: "Jump — During your turn, Kain has flying. Whenever Kain deals combat damage to a
//! player, that player gains control of Kain. If they do, you draw that many cards, create that many tapped Treasure
//! tokens, then lose that much life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("KainTraitorousDragoon.KainTraitorousDragoonEffect", apply).tokens(&["TreasureToken"])];

/// The damaged player (the target pointer) gains control of Kain, still the permanent it was, for the rest of the
/// game (`GainControlTargetEffect(Duration.Custom, true, player)`); then, they control it, the ability's controller
/// draws the damage dealt (`getValue("damage")`, the trigger's X), creates that many tapped Treasures and loses that
/// much life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let o = cx.source_id();
    let zcc = game.obj(o).zcc;
    let now = game.obj(o).controller;
    game.control_changes.push(crate::state::ControlChange {
        target: o,
        zcc,
        to: p,
        before: now,
        until: crate::state::ControlUntil::Forever,
    });
    if now != p {
        set_controller(game, o, p);
    }
    refresh_control(game);
    if game.obj(o).controller != p {
        return Some(());
    }
    let amount = cx.item.x;
    if amount <= 0 {
        return Some(());
    }
    let you = cx.controller();
    draw(game, you, amount as usize);
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: Amount::Fixed(amount),
        tapped_and_attacking: false,
        tapped: true,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    lose_life(game, you, amount);
    Some(())
}
