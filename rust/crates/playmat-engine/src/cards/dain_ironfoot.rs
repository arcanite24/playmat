//! Dain Ironfoot: "When Dain Ironfoot enters, create a colorless Equipment artifact token named Axe with "Equipped
//! creature gets +1/+0" and equip {2}. When you do, attach it to target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("DainIronfoot.DainIronfootCreateAxeEffect", apply)
        .effects(&["new CreateTokenEffect(new AxeToken())"]),
    Handler::effect("DainIronfoot.DainIronfootAttachEffect", attach),
];

/// the Axe, then the reflexive trigger, its target chosen as it goes on the stack, with the Axe as its fixed object
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let Some(axe) = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .find(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
    else {
        return Some(());
    };
    let attach: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": "DainIronfoot.DainIronfootAttachEffect"}))
            .ok()?;
    // `TargetControlledCreaturePermanent`
    let target: TargetDef = serde_json::from_value(serde_json::json!({"class": "TargetPermanent",
        "filter": [{"p": "type", "t": "CREATURE"}, {"p": "controller", "who": "you"}]}))
    .ok()?;
    let delayed = Delayed {
        rule: "When you do, attach it to target creature you control.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![attach],
        source_zcc: cx.source.1,
        fixed: vec![(axe, game.obj(axe).zcc)],
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// `creature.addAttachment(axe)`: the Axe while it's still that permanent
fn attach(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(to) = cx.obj(0) else { return Some(()) };
    let Some(&(axe, zcc)) = cx.item.delayed.as_ref().and_then(|dl| dl.fixed.first()) else { return Some(()) };
    let a = game.obj(axe);
    if a.zone != Zone::Battlefield || a.zcc != zcc || game.obj(to).zone != Zone::Battlefield || to == axe {
        return Some(());
    }
    let z = game.obj(to).zcc;
    game.attach_to(axe, to, z);
    Some(())
}
