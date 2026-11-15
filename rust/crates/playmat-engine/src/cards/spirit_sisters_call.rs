//! Spirit-Sister's Call: "At the beginning of your end step, choose target permanent card in your graveyard. You may
//! sacrifice a permanent that shares a card type with the chosen card. If you do, return the chosen card from your
//! graveyard to the battlefield and it gains "If this permanent would leave the battlefield, exile it instead of
//! putting it anywhere else.""

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SpiritSistersCall.SpiritSistersCallDoIfEffect", apply),
    Handler::effect("SpiritSistersCall.SpiritSistersCallReturnToBattlefieldEffect", return_it),
];

/// With the target still in your graveyard: `DoIfCostPaid(return, SacrificeTargetCost(a permanent you control that
/// shares a card type with it))`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let name = |t: &CardType| match t {
        CardType::Artifact => "ARTIFACT",
        CardType::Battle => "BATTLE",
        CardType::Creature => "CREATURE",
        CardType::Enchantment => "ENCHANTMENT",
        CardType::Instant => "INSTANT",
        CardType::Kindred => "KINDRED",
        CardType::Land => "LAND",
        CardType::Planeswalker => "PLANESWALKER",
        CardType::Sorcery => "SORCERY",
    };
    let types: Vec<serde_json::Value> =
        game.obj(card).def.types.iter().map(|t| serde_json::json!({"p": "type", "t": name(t)})).collect();
    let effect: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "DoIfCostPaid",
        "cost": [{
            "class": "SacrificeTargetCost",
            "filter": [{"p": "or", "xs": types}, {"p": "controller", "who": "you"}]
        }],
        "effect": {"class": "CardEffect", "handler": "SpiritSistersCall.SpiritSistersCallReturnToBattlefieldEffect"}
    }))
    .ok()?;
    cx.run(game, d, &[effect]);
    (!d.stopped()).then_some(())
}

/// The card, still in your graveyard, onto the battlefield under your control; while it stays, it's exiled if it
/// would leave (the same replacement as unearth's, `LeaveBattlefieldExileSourceReplacementEffect`)
fn return_it(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    if !put_cards(game, d, cx.controller(), &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        game.obj_mut(card).unearthed = true;
    }
    Some(())
}
