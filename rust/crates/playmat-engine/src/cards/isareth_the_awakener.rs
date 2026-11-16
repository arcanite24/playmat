//! Isareth the Awakener: "Deathtouch. Whenever Isareth the Awakener attacks, you may pay {X}. When you do, return
//! target creature card with mana value X from your graveyard to the battlefield with a corpse counter on it. If
//! that creature would leave the battlefield, exile it instead of putting it anywhere else."

use super::*;
use crate::card::{CounterDef, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::effect("IsarethTheAwakener.IsarethTheAwakenerCreateReflexiveTriggerEffect", apply),
    Handler::effect("IsarethTheAwakener.IsarethTheAwakenerEffect", return_it),
];

const RULE: &str = "return target creature card with mana value X from your graveyard to the battlefield with a \
                    corpse counter on it. If that creature would leave the battlefield, exile it instead of putting \
                    it anywhere else.";

/// "Pay {X}?", `announceX(0, MAX)` and the cost paid; paid, `fireReflexiveTriggeredAbility` targeting a creature
/// card in your graveyard with mana value X
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Pay {X}?")? {
        return Some(());
    }
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: i32::MAX }) {
            Answer::Int(n) if n >= 0 => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let cost = ManaCost { generic: x as u32, ..ManaCost::default() };
    let before = game.clone();
    if !pay_mana(game, d, you, &cost) {
        if d.stopped() {
            return None;
        }
        *game = before;
        return Some(());
    }
    let target: TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetCardInGraveyard",
        "filter": [
            {"p": "type", "t": "CREATURE"},
            {"p": "mv", "cmp": "EQUAL_TO", "n": x},
            {"p": "owner", "who": "you"}
        ]
    }))
    .ok()?;
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "CardEffect", "handler": "IsarethTheAwakener.IsarethTheAwakenerEffect"
    }))
    .ok()?;
    let delayed = Delayed {
        rule: RULE.to_string(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), x, ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// the target onto the battlefield under your control with a corpse counter as it enters (`setEnterWithCounters`);
/// then, while it stays, it's exiled if it would leave (`LeaveBattlefieldExileTargetReplacementEffect`, unearth's)
fn return_it(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    let back = EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect {
        tapped: false,
        owner: false,
        counters: Some(CounterDef { name: "corpse".into(), amount: 1 }),
        attacking: false,
    };
    cx.run_on(game, d, &[back], vec![Target::Obj(card)]);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        game.obj_mut(card).unearthed = true;
    }
    Some(())
}
