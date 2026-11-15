//! Back for More: "Return target creature card from your graveyard to the battlefield. When you do, it fights up to
//! one target creature you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BackForMore.BackForMoreEffect", apply).filters(&["StaticFilters.FILTER_CREATURE_YOU_DONT_CONTROL"]),
    Handler::effect("BackForMore.BackForMoreDamageEffect", fight),
];

/// The card onto the battlefield under your control; there, `fireReflexiveTriggeredAbility` of the fight, with up to
/// one target creature you don't control, the permanent kept as its fixed object (`MageObjectReference`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    put_onto_battlefield(game, d, card, you);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let target = TargetDef::TargetPermanent {
        filter: cx.filters.first()?.clone(),
        min: 0,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let e: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": "BackForMore.BackForMoreDamageEffect"}))
            .ok()?;
    let delayed = Delayed {
        rule: "It fights up to one target creature you don't control.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: vec![(card, game.obj(card).zcc)],
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// `BackForMoreDamageEffect`: the returned permanent, still the same object, fights the target (`Permanent.fight`:
/// it's dealt the target's power first), both still creatures on the battlefield
fn fight(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(&(a, zcc)) = cx.item.delayed.as_ref().and_then(|dl| dl.fixed.first()) else { return Some(()) };
    let Some(b) = cx.obj(0) else { return Some(()) };
    let creature = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
    if game.obj(a).zcc == zcc && creature(game, a) && creature(game, b) {
        let pb = game.power_toughness(b).0;
        let pa = game.power_toughness(a).0;
        deal_damage(game, b, Target::Obj(a), pb, false);
        let pa = pa.min(fight_lethal(game, b, a));
        deal_damage(game, a, Target::Obj(b), pa, false);
    }
    Some(())
}
