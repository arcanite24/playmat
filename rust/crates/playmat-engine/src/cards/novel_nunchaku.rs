//! Novel Nunchaku: "When this Equipment enters, attach it to target creature you control. When you do, equipped
//! creature fights up to one target creature an opponent controls. Equipped creature gets +1/+1 and has trample.
//! Equip {3}"

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("NovelNunchaku.NovelNunchakuEffect", apply)
        .effects(&["new NovelNunchakuFightEffect()"])
        .filters(&["StaticFilters.FILTER_OPPONENTS_PERMANENT_A_CREATURE"]),
    Handler::effect("NovelNunchaku.NovelNunchakuFightEffect", fight),
];

/// With the creature it attached to still there (`source.getFirstTarget()`), `fireReflexiveTriggeredAbility`: "equipped
/// creature fights up to one target creature an opponent controls", fixed at that creature
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(creature) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let target = TargetDef::TargetPermanent {
        filter: cx.filters[0].clone(),
        min: 0,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "Equipped creature fights up to one target creature an opponent controls.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: vec![(creature, game.obj(creature).zcc)],
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// `equippedCreature.fight(target)`: both still creatures on the battlefield, the target deals its power to the
/// fixed creature, then the fixed creature its power to the target (`fightWithExcess`)
fn fight(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let fixed = cx.item.delayed.as_ref().and_then(|dl| dl.fixed.first().copied());
    let Some((a, zcc)) = fixed else { return Some(()) };
    let Some(b) = cx.obj(0) else { return Some(()) };
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
