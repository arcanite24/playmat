//! Devastating Onslaught: "Create X tokens that are copies of target artifact or creature you control. Those tokens
//! gain haste until end of turn. Sacrifice them at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DevastatingOnslaught.DevastatingOnslaughtEffect", apply)];

/// With the target still there and X 1 or more: X copies of it with haste (`CreateTokenCopyTargetEffect(…, true, X)`
/// gives the copies haste), then `sacrificeTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let x = cx.item.x;
    if game.obj(o).zone != Zone::Battlefield || x < 1 {
        return Some(());
    }
    let made = token_copy_modified(game, d, o, cx.controller(), x as u32, &|def: &mut CardDef| {
        def.abilities.push(AbilityDef::HasteAbility);
    });
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = made.into_iter().map(|t| (t, game.obj(t).zcc)).collect();
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, sacrifice the token copies.".into(),
        sacrifice: made,
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
