//! Sandbender Scavengers: "Whenever you sacrifice another permanent, put a +1/+1 counter on this creature. When this
//! creature dies, you may exile it. When you do, return target creature card with mana value less than or equal to
//! this creature's power from your graveyard to the battlefield."

use super::*;
use crate::card::TargetDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("SandbenderScavengers.SandbenderScavengersEffect", apply)];

/// The card, still where it died to: "Exile <name>?", exiled, then a reflexive trigger returning a creature card
/// with mana value up to its last known power from your graveyard waits to go on the stack
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = cx.source_id();
    if game.obj(card).zone != Zone::Graveyard || game.obj(card).token {
        return Some(());
    }
    let name = game.obj(card).def.name.clone();
    if !choose_use(game, d, you, &format!("Exile {name}?"))? {
        return Some(());
    }
    let mv = game.obj(card).power_lki.unwrap_or(0);
    game.move_to(card, Zone::Exile);
    let target: TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetCardInGraveyard",
        "filter": [{"p": "owner", "who": "you"}, {"p": "type", "t": "CREATURE"}, {"p": "mv", "cmp": "FEWER_THAN", "n": mv + 1}],
    }))
    .ok()?;
    let effect: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "ReturnFromGraveyardToBattlefieldTargetEffect"})).ok()?;
    let delayed = Delayed {
        rule: format!("Return target creature card with mana value {mv} or less from your graveyard to the battlefield."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(card, you) });
    Some(())
}
