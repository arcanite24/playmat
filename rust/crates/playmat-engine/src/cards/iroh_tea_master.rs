//! Iroh, Tea Master: "When Iroh enters, create a Food token. At the beginning of combat on your turn, you may have
//! target opponent gain control of target permanent you control. When you do, create a 1/1 white Ally creature
//! token. Put a +1/+1 counter on that token for each permanent you own that your opponents control."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("IrohTeaMaster.IrohTeaMasterControlEffect", control),
    Handler::effect("IrohTeaMaster.IrohTeaMasterTokenEffect", token).tokens(&["AllyToken"]),
];

/// The opponent (tag 1) gains control of the permanent (tag 2) for good (`GainControlTargetEffect(Duration.Custom,
/// true, player)` on a `FixedTarget`), then the reflexive trigger
fn control(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let Some(o) = cx.obj(1) else { return Some(()) };
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
    let e: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": "IrohTeaMaster.IrohTeaMasterTokenEffect"}))
            .ok()?;
    let delayed = Delayed {
        rule: "Create a 1/1 white Ally creature token. Put a +1/+1 counter on that token for each permanent you own that your opponents control."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// The Ally token, then (counted after it entered) a +1/+1 counter on it for each permanent you own that an
/// opponent controls
fn token(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    if d.stopped() {
        return None;
    }
    let count = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).owner == you && game.obj(**o).controller != you)
        .count() as i32;
    if count < 1 {
        return Some(());
    }
    let made: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    for t in made {
        put_counters(game, t, "+1/+1", count, you);
    }
    Some(())
}
