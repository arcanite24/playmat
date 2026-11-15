//! Yotia Declares War: "II — Tap any number of untapped artifacts you control. When you do, Yotia Declares War deals
//! that much damage to target creature or planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("YotiaDeclaresWar.YotiaDeclaresWarEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE_OR_PLANESWALKER"])];

/// Any number of your untapped artifacts (`TargetPermanent(0, MAX, …, true)` chosen with `choose`), each tapped;
/// then, if any, `fireReflexiveTriggeredAbility`: a reflexive trigger dealing that much damage to its own target
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| {
            let o = game.obj(*p);
            o.controller == you && !o.tapped && game.has_type(*p, CardType::Artifact)
        })
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, usize::MAX, &rule)?;
    let chosen: Vec<ObjId> = chosen.into_iter().filter(|p| game.obj(*p).zone == Zone::Battlefield).collect();
    if chosen.is_empty() {
        return Some(());
    }
    for p in &chosen {
        tap(game, *p);
    }
    let n = chosen.len() as i32;
    let target = TargetDef::TargetPermanent {
        filter: cx.filters[0].clone(),
        min: 1,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: format!("{{this}} deals {n} damage to target creature or planeswalker."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(n) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
