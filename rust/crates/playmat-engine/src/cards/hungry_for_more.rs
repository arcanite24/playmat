//! Hungry for More: "Create a 3/1 black and red Vampire creature token with trample, lifelink, and haste. Sacrifice it
//! at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("HungryForMore.HungryForMoreEffect", apply).tokens(&["HungryForMoreVampireToken"])];

/// the token, then a delayed "sacrifice that token" on the tokens made (`FixedTargets` of the last added ids)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| !before.contains(o) && game.obj(*o).token)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = Delayed {
        rule: "At the beginning of the next end step, sacrifice that token.".into(),
        sacrifice: made,
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let item = StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) };
    game.at_next_end_step.push(item);
    Some(())
}
