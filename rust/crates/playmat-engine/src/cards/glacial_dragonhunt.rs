//! Glacial Dragonhunt: "Draw a card, then you may discard a card. When you discard a nonland card this way, Glacial
//! Dragonhunt deals 3 damage to target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GlacialDragonhunt.GlacialDragonhuntEffect", apply)
    .effects(&["new DamageTargetEffect(3)"])
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])];

/// `discard(0, 1)` (nothing asked of an empty hand), then on a nonland card `fireReflexiveTriggeredAbility`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.is_empty() {
        return None;
    }
    let chosen = choose_cards(game, d, you, &hand, &hand, 0, 1, "discard")?;
    for c in &chosen {
        discard_card(game, you, *c);
    }
    game.discarded_cards();
    if !chosen.iter().any(|c| !game.obj(*c).def.types.contains(&CardType::Land)) {
        return None;
    }
    let target = TargetDef::TargetPermanent {
        filter: cx.filters[0].clone(),
        min: 1,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "{this} deals 3 damage to target creature.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
