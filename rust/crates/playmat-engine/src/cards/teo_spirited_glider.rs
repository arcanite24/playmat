//! Teo, Spirited Glider: "Flying. Whenever one or more creatures you control with flying attack, draw a card, then
//! discard a card. When you discard a nonland card this way, put a +1/+1 counter on target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TeoSpiritedGlider.TeoSpiritedGliderEffect", apply)
    .effects(&["new AddCountersTargetEffect(CounterType.P1P1.createInstance())"])];

/// `discardOne` (nothing with an empty hand); a nonland card discarded, the reflexive trigger
/// (`TargetControlledCreaturePermanent`) waits for the stack
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    let Some(&card) = discarded.first() else { return Some(()) };
    if game.obj(card).is(CardType::Land) {
        return Some(());
    }
    let target: crate::card::TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetPermanent", "filter": [{"p": "type", "t": "CREATURE"}, {"p": "controller", "who": "you"}]
    }))
    .ok()?;
    let delayed = crate::state::Delayed {
        rule: "put a +1/+1 counter on target creature you control.".into(),
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
