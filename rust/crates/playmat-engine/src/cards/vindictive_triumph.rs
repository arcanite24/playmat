//! Vindictive Triumph: "Exile target creature or planeswalker. If that permanent's mana value was 3 or less, return
//! it to the battlefield tapped under your control. Exile it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VindictiveTriumph.VindictiveTriumphEffect", apply)];

/// `ReturnToBattlefieldUnderYourControlTargetEffect(true, true)`: the target, exiled since it was chosen (the target
/// pointer's one zone change), back tapped under your control; then, back on the battlefield, an "at the beginning of
/// the next end step, exile that permanent" pointing at it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (Some(Target::Obj(o)), Some(&z)) = (
        cx.item.targets.first().and_then(|ts| ts.first()).copied(),
        cx.item.target_zcc.first().and_then(|zs| zs.first()),
    ) else {
        return Some(());
    };
    let c = game.obj(o);
    if c.zone != Zone::Exile || c.token || c.zcc != z + 1 {
        return Some(());
    }
    enter_battlefield(game, d, o, you, false);
    if d.stopped() {
        return None;
    }
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    game.obj_mut(o).tapped = true;
    let delayed = Delayed {
        rule: "At the beginning of the next end step, exile that permanent.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: vec![(o, game.obj(o).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
