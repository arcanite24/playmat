//! Valduk, Keeper of the Flame: "At the beginning of combat on your turn, for each Aura and Equipment attached to
//! Valduk, create a 3/1 red Elemental creature token with trample and haste. Exile those tokens at the beginning of
//! the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ValdukKeeperOfTheFlame.ValdukKeeperOfTheFlameEffect", apply)
    .effects(&["new CreateTokenEffect(new Elemental31TrampleHasteToken())"])];

/// Only while Valduk is on the battlefield (`game.getPermanent`): the Equipment and then the Auras attached to it
/// (`EquipmentAttachedCount`, `AuraAttachedCount`), that many tokens, and `exileTokensCreatedAtNextEndStep` (made even
/// for no tokens: `FixedTargets` of none)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield {
        return Some(());
    }
    let attached: Vec<ObjId> = game.attachments(src).collect();
    let n = attached.iter().filter(|a| game.has_subtype(**a, "Equipment")).count()
        + attached.iter().filter(|a| game.has_subtype(**a, "Aura")).count();
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
        *amount = crate::card::Amount::Fixed(n as i32);
    }
    let before = game.objs.len();
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, exile those tokens.".into(),
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
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, cx.controller()) });
    Some(())
}
