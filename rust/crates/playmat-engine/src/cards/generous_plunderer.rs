//! Generous Plunderer: "Menace. At the beginning of your upkeep, you may create a Treasure token. When you do, target
//! opponent creates a tapped Treasure token. Whenever Generous Plunderer attacks, it deals damage to defending player
//! equal to the number of artifacts they control."

use super::*;
use crate::card::{Amount, TokenDef};

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GenerousPlunderer.GenerousPlundererEffect", apply).tokens(&["TreasureToken"]),
    // the reflexive trigger's `CreateTokenTargetEffect(new TreasureToken(), 1, true)` (cards.json has no tapped one)
    Handler::effect("GenerousPlunderer.GenerousPlundererTappedTreasureEffect", tapped_treasure),
];

fn treasure(token: &TokenDef, tapped: bool) -> EffectDef {
    EffectDef::CreateTokenEffect {
        token: token.clone(),
        amount: Amount::Fixed(1),
        tapped_and_attacking: false,
        tapped,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    }
}

/// (the trigger asked "you may") the Treasure, then `fireReflexiveTriggeredAbility`: a reflexive trigger targeting
/// an opponent, who creates a tapped Treasure
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let token = cx.tokens.first()?;
    cx.run(game, d, &[treasure(token, false)]);
    if d.stopped() {
        return None;
    }
    let handler = serde_json::from_value(serde_json::json!("GenerousPlunderer.GenerousPlundererTappedTreasureEffect")).ok()?;
    let effect =
        EffectDef::CardEffect { handler, args: Vec::new(), tokens: vec![token.clone()], effects: Vec::new(), filters: Vec::new() };
    let delayed = Delayed {
        rule: "target opponent creates a tapped Treasure token.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetOpponent],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// the target opponent creates a tapped Treasure
fn tapped_treasure(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    cx.run_as(game, d, &[treasure(cx.tokens.first()?, true)], p);
    Some(())
}
