//! Urabrask's Forge: "At the beginning of combat on your turn, put an oil counter on Urabrask's Forge, then create an
//! X/1 red Phyrexian Horror creature token with trample and haste, where X is the number of oil counters on Urabrask's
//! Forge. Sacrifice that token at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("UrabrasksForge.UrabrasksForgeEffect", apply).tokens(&["PhyrexianHorrorRedToken"])];

/// X from the source permanent (or as it last was), then the token and its delayed sacrifice ("sacrifice it")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let x = game.obj(src).counter("oil");
    let mut token = cx.tokens[0].clone();
    token.power = x;
    let before = game.objs.len();
    cx.create_tokens(game, d, &token, 1);
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, sacrifice it.".into(),
        sacrifice: made,
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, cx.controller()) });
    Some(())
}
