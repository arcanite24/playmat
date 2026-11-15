//! Miasma Demon: "Flying. When Miasma Demon enters, you may discard any number of cards. When you do, up to that many
//! target creatures each get -2/-2 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MiasmaDemon.MiasmaDemonEffect", apply)
    .effects(&["new BoostTargetEffect(-2, -2)"])
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])];

/// `discard(0, MAX)` (nothing asked of an empty hand); one or more discarded, `fireReflexiveTriggeredAbility` with
/// up to that many target creatures
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &hand, &hand, 0, hand.len(), "discard")?;
    for c in &chosen {
        discard_card(game, you, *c);
    }
    game.discarded_cards();
    if chosen.is_empty() {
        return Some(());
    }
    let target = TargetDef::TargetPermanent {
        filter: cx.filters.first()?.clone(),
        min: 0,
        max: chosen.len(),
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "Up to that many target creatures each get -2/-2 until end of turn.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
