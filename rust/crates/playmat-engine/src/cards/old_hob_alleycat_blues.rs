//! Old Hob, Alleycat Blues: "At the beginning of combat on your turn, create a 2/2 red Mutant creature token. It
//! gains haste until end of turn. Destroy it at the beginning of the next end step. {1}{W}: Target attacking creature
//! token gains indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OldHobAlleycatBlues.OldHobAlleycatBluesEffect", apply)
    .effects(&["new CreateTokenEffect(new RedMutantToken())"])];

/// The token, haste for it until end of turn, then a delayed "destroy it" at the beginning of the next end step
/// (`AtTheBeginOfNextEndStepDelayedTriggeredAbility` with no trigger phrase: its text is the effect's)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    for (t, _) in &made {
        grant(game, *t, &AbilityDef::HasteAbility);
    }
    let delayed = crate::state::Delayed {
        rule: "destroy it at the beginning of the next end step.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DestroyTargetEffect],
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
