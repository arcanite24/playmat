//! Reno and Rude: "Menace. Whenever Reno and Rude deals combat damage to a player, exile the top card of that
//! player's library. Then you may sacrifice another creature or artifact. If you do, you may play the exiled card
//! this turn, and mana of any type can be spent to cast it."

use super::*;
use crate::card::CostDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("RenoAndRude.RenoAndRudeEffect", apply)];

/// that player's top card exiled; then, if the `SacrificeTargetCost` can be paid, "Sacrifice another creature or
/// artifact?" and the sacrifice; paid: the card is playable by you this turn with mana of any type
/// (`makeCardPlayable`). With no card, the sacrifice is still offered.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let card = cx.player(0).and_then(|p| game.player(p).library.first().copied());
    if let Some(c) = card {
        game.move_to(c, Zone::Exile);
    }
    // `FILTER_CONTROLLED_ARTIFACT_OR_OTHER_CREATURE`: an artifact or another creature you control
    let filter: Filter = serde_json::from_value(serde_json::json!([
        {"p": "or", "xs": [
            {"p": "type", "t": "ARTIFACT"},
            {"p": "and", "xs": [{"p": "another"}, {"p": "type", "t": "CREATURE"}]}
        ]},
        {"p": "controller", "who": "you"}
    ]))
    .ok()?;
    let cost = CostDef::SacrificeTargetCost { filter, amount: 1 };
    let paid = can_pay_cost(game, you, src, &cost)
        && choose_use(game, d, you, "Sacrifice another creature or artifact?")?
        && pay_cost(game, d, you, src, &cost);
    if d.stopped() {
        return None;
    }
    if let Some(c) = card.filter(|c| paid && game.obj(*c).zone == Zone::Exile) {
        let turn = game.turn;
        let o = game.obj_mut(c);
        o.may_play = Some((you, turn));
        o.any_mana = true;
    }
    Some(())
}
