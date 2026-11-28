//! Flamerush Rider: "Whenever Flamerush Rider attacks, create a token that's tapped and attacking that's a copy of
//! another target attacking creature. Exile the token at end of combat."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FlamerushRider.FlamerushRiderEffect", apply)
    .effects(&["new CreateTokenCopyTargetEffect(null, null, true, 1, true, true)"])];

/// `CreateTokenCopyTargetEffect(you, null, true, 1, true, true)` of the target (or as it last was), then
/// `removeTokensCreatedAt(…, exile, END_COMBAT, ANY)`: each token exiled at end of combat
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    let first = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(of)]);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    if made.is_empty() {
        return Some(());
    }
    let delayed = Delayed {
        rule: "At end of combat, exile the token copies.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: 0,
        fixed: made.iter().map(|t| (*t, game.obj(*t).zcc)).collect(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_end_of_combat.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
