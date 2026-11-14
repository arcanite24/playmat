//! Thorin, Mountain-king: "When Thorin enters, attach any number of target Equipment you control to target creature
//! you control. When one or more Equipment become attached to that creature this way, that creature deals damage
//! equal to its power to up to one target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ThorinMountainKing.ThorinMountainKingEffect", apply),
    Handler::effect("ThorinMountainKing.ThorinMountainKingDamageEffect", damage),
];

/// each target Equipment still there attached to the target creature (one already attached to it doesn't count, as
/// `addAttachment` returns false); with any attached, the reflexive trigger, fixed on the creature
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(creature) = cx.obj(1).filter(|c| game.obj(*c).zone == Zone::Battlefield) else { return Some(()) };
    let mut attached = 0;
    for e in cx.objs(0) {
        if game.obj(e).zone != Zone::Battlefield
            || !game.has_subtype(e, "Equipment")
            || e == creature
            || game.attached(e) == Some(creature)
        {
            continue;
        }
        let zcc = game.obj(creature).zcc;
        game.attach_to(e, creature, zcc);
        attached += 1;
    }
    if attached == 0 {
        return Some(());
    }
    let effect: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "CardEffect", "handler": "ThorinMountainKing.ThorinMountainKingDamageEffect"
    }))
    .ok()?;
    // `TargetCreaturePermanent(0, 1)`
    let target: TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetPermanent", "filter": [{"p": "type", "t": "CREATURE"}], "min": 0, "max": 1
    }))
    .ok()?;
    let delayed = Delayed {
        rule: "that creature deals damage equal to its power to up to one target creature".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed: vec![(creature, game.obj(creature).zcc)],
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// the creature, while it's still on the battlefield (any object it is now: `game.getPermanent(creatureId)`), deals
/// damage equal to its power (0 or more) to the target creature
fn damage(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(&(creature, _)) = cx.item.delayed.as_ref().and_then(|dl| dl.fixed.first()) else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(creature).0.max(0);
    for t in cx.objs(0) {
        if game.obj(t).zone == Zone::Battlefield {
            deal_damage(game, creature, Target::Obj(t), power, false);
        }
    }
    Some(())
}
