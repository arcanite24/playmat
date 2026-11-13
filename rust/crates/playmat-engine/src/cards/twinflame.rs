//! Twinflame: "Choose any number of target creatures you control. For each of them, create a token that's a copy of
//! that creature, except it has haste. Exile those tokens at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Twinflame.TwinflameCopyEffect", apply)];

/// A haste copy of each target still legal, in target order; then one delayed exile for all the tokens made (made
/// even with none: `FixedTargets` of none)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut made = Vec::new();
    for o in cx.objs(0) {
        for t in token_copy_with(game, d, o, cx.controller(), None, true) {
            made.push((t, game.obj(t).zcc));
        }
        if d.stopped() {
            return None;
        }
    }
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, exile that permanent.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: made,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
