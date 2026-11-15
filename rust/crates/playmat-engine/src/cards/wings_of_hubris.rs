//! Wings of Hubris: "Equipped creature has flying. Sacrifice Wings of Hubris: Equipped creature can't be blocked this
//! turn. Sacrifice it at the beginning of the next end step. Equip {1}"

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("WingsOfHubris.WingsOfHubrisEffect", apply)
        .effects(&["new CantBeBlockedTargetEffect(Duration.EndOfTurn)"]),
    // the delayed trigger's `SacrificeTargetEffect("sacrifice this", controller)`
    Handler::effect("WingsOfHubris.SacrificeTargetEffect", sacrifice_it),
];

/// the creature the Equipment was attached to as it was last on the battlefield: can't be blocked this turn, and an
/// "at the beginning of the next end step" sacrifice of it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (src, zcc) = cx.source;
    let attached = if game.obj(src).zcc == zcc { game.obj(src).attached_to } else { game.obj(src).lki_attached_to };
    let Some((o, o_zcc)) = attached else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != o_zcc {
        return Some(());
    }
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(o)]);
    if d.stopped() {
        return None;
    }
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "CardEffect", "handler": "WingsOfHubris.SacrificeTargetEffect"
    }))
    .ok()?;
    let delayed = Delayed {
        rule: "At the beginning of the next end step, sacrifice this.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: zcc,
        fixed: vec![(o, o_zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, cx.controller()) });
    Some(())
}

/// the creature, when the ability's controller still controls it
fn sacrifice_it(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield && game.obj(o).controller == cx.controller() {
            sacrifice(game, o);
        }
    }
    Some(())
}
