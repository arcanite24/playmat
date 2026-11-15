//! Curse of the Werefox: "Create a Monster Role token attached to target creature you control. When you do, that
//! creature fights up to one target creature you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("CurseOfTheWerefox.CurseOfTheWerefoxEffect", apply)
        .effects(&["new CreateRoleAttachedTargetEffect(RoleType.MONSTER)"])
        .filters(&["StaticFilters.FILTER_CREATURE_YOU_DONT_CONTROL"]),
    Handler::effect("CurseOfTheWerefox.CurseOfTheWerefoxFightEffect", fight),
];

/// The Role on the target (`RoleType.MONSTER.createToken`); with one made, `fireReflexiveTriggeredAbility`: up to one
/// target creature you don't control, chosen as it goes on the stack, the creature fixed as "that creature"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let before = game.objs.len();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let made = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .any(|o| game.obj(o).token && game.obj(o).zone == Zone::Battlefield);
    if !made {
        return Some(());
    }
    let fight: EffectDef = serde_json::from_value(
        serde_json::json!({"class": "CardEffect", "handler": "CurseOfTheWerefox.CurseOfTheWerefoxFightEffect"}),
    )
    .ok()?;
    let target = TargetDef::TargetPermanent {
        filter: cx.filters.first()?.clone(),
        min: 0,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "that creature fights up to one target creature you don't control.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![fight],
        source_zcc: cx.source.1,
        fixed: vec![(creature, game.obj(creature).zcc)],
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// "that creature" (still that permanent) and the target, both creatures: each is dealt the other's power, the
/// second only lethal damage (`fightWithExcess`)
fn fight(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let b = cx.obj(0)?;
    let &(a, zcc) = cx.item.delayed.as_ref()?.fixed.first()?;
    let creature = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
    if game.obj(a).zcc != zcc || !creature(game, a) || !creature(game, b) {
        return Some(());
    }
    let pb = game.power_toughness(b).0;
    let pa = game.power_toughness(a).0;
    deal_damage(game, b, Target::Obj(a), pb, false);
    let pa = pa.min(fight_lethal(game, b, a));
    deal_damage(game, a, Target::Obj(b), pa, false);
    Some(())
}
