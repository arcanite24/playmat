//! Flameshadow Conjuring: "Whenever a nontoken creature you control enters, you may pay {R}. If you do, create a
//! token that's a copy of that creature. That token gains haste. Exile it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FlameshadowConjuring.FlameshadowConjuringEffect", apply)];

/// `CreateTokenCopyTargetEffect(null, null, true)` of the creature that entered, then for each token it made an
/// "at the beginning of the next end step" exile (`AtTheBeginOfNextEndStepDelayedTriggeredAbility`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let it = cx.item.entering.or_else(|| cx.obj(0))?;
    let e: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CreateTokenCopyTargetEffect", "haste": true})).ok()?;
    let first = game.objs.len();
    cx.run_on(game, d, &[e], vec![Target::Obj(it)]);
    let made: Vec<(ObjId, u32)> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    for t in made {
        let delayed = Delayed {
            rule: "At the beginning of the next end step, exile that permanent.".into(),
            sacrifice: Vec::new(),
            warp_exile: Vec::new(),
            effects: vec![EffectDef::ExileTargetEffect],
            source_zcc: cx.source.1,
            fixed: vec![t],
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_next_end_step
            .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    }
    Some(())
}
