//! The Last Ronin: "I — Destroy all creatures. II — Mill four cards. When you do, return target creature card from
//! your graveyard to your hand. III — Whenever a creature you control attacks alone this turn, put three +1/+1
//! counters on it. It gains trample, lifelink, and indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TheLastRonin.TheLastRoninEffect", mill_then_return)
        .effects(&["new MillCardsControllerEffect(4)", "new ReturnFromGraveyardToHandTargetEffect()"]),
    Handler::trigger(
        "TheLastRonin.TheLastRoninTriggeredAbility",
        "new AttacksAloneControlledTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance(3)).withTargetDescription(\"it\"), true, false)",
        any_alone,
    ),
];

/// mills four, then `fireReflexiveTriggeredAbility`: a reflexive trigger with its own target, waiting to go on the
/// stack (it's fired whatever was milled)
fn mill_then_return(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    // (`TargetCardInYourGraveyard(StaticFilters.FILTER_CARD_CREATURE_YOUR_GRAVEYARD)`)
    let target: TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetCardInGraveyard",
        "filter": [{"p": "type", "t": "CREATURE"}, {"p": "owner", "who": "you"}],
    }))
    .ok()?;
    let delayed = Delayed {
        rule: "return target creature card from your graveyard to your hand.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects[1..].to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// a creature you control attacking alone (`ATTACKER_DECLARED` with `attacksAlone`), pointed at: the base is all of it
fn any_alone(_: &Game, _: &StackItem) -> bool {
    true
}
