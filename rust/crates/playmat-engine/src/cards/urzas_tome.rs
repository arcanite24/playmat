//! Urza's Tome: "{3}, {T}: Draw a card. Then discard a card unless you exile a historic card from your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UrzasTome.UrzasTomeEffect", apply)];

/// asked first (whether or not there's a historic card); an exiled one saves the discard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cost: CostDef = serde_json::from_value(serde_json::json!({
        "class": "ExileFromGraveCost",
        "amount": 1,
        "filter": [{"p": "or", "xs": [
            {"p": "type", "t": "ARTIFACT"}, {"p": "supertype", "s": "LEGENDARY"}, {"p": "subtype", "s": "Saga"}
        ]}]
    }))
    .ok()?;
    if choose_use(game, d, you, "Exile a historic card from your graveyard?")?
        && can_pay_cost(game, you, cx.source_id(), &cost)
        && pay_cost(game, d, you, cx.source_id(), &cost)
    {
        return Some(());
    }
    if d.stopped() {
        return None;
    }
    discard(game, d, you, 1);
    Some(())
}
