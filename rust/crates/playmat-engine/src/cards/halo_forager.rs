//! Halo Forager: "Flying. When this creature enters, you may pay {X}. When you do, you may cast target instant or
//! sorcery card with mana value X from a graveyard without paying its mana cost. If that spell would be put into a
//! graveyard, exile it instead."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::effect("HaloForager.HaloForagerPayEffect", apply)
    .effects(&["new MayCastTargetCardEffect(CastManaAdjustment.WITHOUT_PAYING_MANA_COST, true)"])];

const RULE: &str = "When you do, you may cast target instant or sorcery card with mana value X from a graveyard \
                    without paying its mana cost. If that spell would be put into a graveyard, exile it instead.";

/// Isareth the Awakener's "Pay {X}?", `announceX(0, MAX)` and the cost paid; paid, `fireReflexiveTriggeredAbility`
/// targeting an instant or sorcery card with mana value X in any graveyard
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
            {"p": "or", "xs": [{"p": "type", "t": "INSTANT"}, {"p": "type", "t": "SORCERY"}]},
            {"p": "mv", "cmp": "EQUAL_TO", "n": x}
        ]
    }))
    .ok()?;
    let delayed = Delayed {
        rule: RULE.to_string(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), x, ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
