//! Ill-Timed Explosion: "Draw two cards. Then you may discard two cards. When you do, Ill-Timed Explosion deals X
//! damage to each creature, where X is the greatest mana value among cards discarded this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IllTimedExplosion.IllTimedExplosionEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])];

/// With two or more cards in hand, "Discard two cards?"; yes: `TargetDiscard(2)` chosen with `Player.choose`, the
/// two discarded (`discard(cards, …)`, in the order chosen), then a reflexive `DamageAllEffect(X, creatures)` with X
/// the greatest mana value among them
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.len() < 2 || !choose_use(game, d, you, "Discard two cards?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &hand, &hand, 2, 2, &rule)?;
    let x = chosen.iter().map(|c| game.obj(*c).def.mana_value() as i32).max().unwrap_or(0);
    for c in &chosen {
        discard_card(game, you, *c);
    }
    game.discarded_cards();
    let delayed = Delayed {
        rule: format!("{{this}} deals {x} damage to each creature."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageAllEffect { amount: Amount::Fixed(x), filter: cx.filters[0].clone() }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
