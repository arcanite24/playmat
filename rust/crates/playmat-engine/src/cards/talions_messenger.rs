//! Talion's Messenger: "Flying. Whenever you attack with one or more Faeries, draw a card, then discard a card. When
//! you discard a card this way, put a +1/+1 counter on target Faerie you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TalionsMessenger.TalionsMessengerEffect", apply)
    .effects(&["new AddCountersTargetEffect(CounterType.P1P1.createInstance())"])
    .filters(&["new FilterPermanent(SubType.FAERIE, \"Faerie you control\")"])];

/// draw, discard, and with a card discarded the reflexive trigger putting a +1/+1 counter on target Faerie
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    let gone = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    if gone.is_empty() {
        return Some(());
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
        rule: "put a +1/+1 counter on target Faerie you control.".into(),
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
