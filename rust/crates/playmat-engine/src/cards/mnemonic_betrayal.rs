//! Mnemonic Betrayal: "Exile all cards from all opponents' graveyards. You may cast those cards this turn, and you may
//! spend mana as though it were mana of any type to cast those spells. At the beginning of the next end step, if any
//! of those cards remain exiled, return them to their owners' graveyards."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("MnemonicBetrayal.MnemonicBetrayalExileEffect", exile),
    Handler::effect("MnemonicBetrayal.MnemonicBetrayalReturnEffect", back),
    Handler::condition("MnemonicBetrayal.MnemonicBetrayalDelayedTriggeredAbility", remain),
];

const RETURN: &str = "MnemonicBetrayal.MnemonicBetrayalReturnEffect";
const REMAIN: &str = "MnemonicBetrayal.MnemonicBetrayalDelayedTriggeredAbility";

/// Every opponent's graveyard (in turn order from you) exiled together into this spell's zone; the nonland ones
/// castable by you this turn with mana of any type (`makeCardPlayable(…, true, EndOfTurn, true)`); then the delayed
/// "at the beginning of the next end step", which remembers those exiled
fn exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let _ = d;
    let you = cx.controller();
    let mut cards: Vec<ObjId> = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if !game.player(p).lost {
            cards.extend(game.player(p).graveyard.iter().copied());
        }
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let src = cx.source_id();
    let turn = game.turn;
    let exiled: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    for c in &exiled {
        game.exiled_for_source.push((src, cx.source.1, *c));
        if !game.obj(*c).is(CardType::Land) {
            let o = game.obj_mut(*c);
            o.may_play = Some((you, turn));
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.cast_only = true;
            o.any_mana = true;
        }
    }
    let effect: EffectDef = serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": RETURN})).ok()?;
    let condition: crate::card::Condition =
        serde_json::from_value(serde_json::json!({"c": "card", "handler": REMAIN})).ok()?;
    let delayed = Delayed {
        rule: "At the beginning of the next end step, if any of those cards remain exiled, return them to their \
               owner's graveyards."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: Some(condition),
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, you) });
    Some(())
}

/// The cards exiled with this spell that are still in exile
fn remaining(game: &Game, source: ObjId) -> Vec<ObjId> {
    game.exiled_for_source
        .iter()
        .filter(|(s, _, o)| *s == source && game.obj(*o).zone == Zone::Exile)
        .map(|(_, _, o)| *o)
        .collect()
}

/// `checkInterveningIfClause`: any of those cards still exiled
fn remain(game: &Game, _you: Seat, source: ObjId) -> bool {
    !remaining(game, source).is_empty()
}

/// Those still exiled, to their owners' graveyards together
fn back(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let cards = remaining(game, src);
    game.exiled_for_source.retain(|(s, _, _)| *s != src);
    for c in cards {
        game.move_to(c, Zone::Graveyard);
    }
    Some(())
}
