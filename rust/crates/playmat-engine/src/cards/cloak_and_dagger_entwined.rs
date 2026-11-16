//! Cloak and Dagger, Entwined: "Deathtouch, lifelink. When Cloak and Dagger enter, choose target opponent and up to one
//! target creature they control. They reveal their hand. You may exile a nonland card from their hand or the chosen
//! creature until Cloak and Dagger leave the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CloakAndDaggerEntwined.CloakAndDaggerEntwinedEffect", apply)];

/// The opponent reveals their hand; with this still on the battlefield and a nonland card there or the creature,
/// "Exile a card or creature?", then (with both) "Choose what to exile" (yes: Hand); the card chosen from their hand
/// is exiled until this leaves, back to their hand then (`ExileUntilSourceLeavesEffect(Zone.HAND)`), or the creature
/// is, back to the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let hand = game.player(opponent).hand.clone();
    if !hand.is_empty() {
        let (src, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
        let shown = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: shown });
    }
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let creature = cx.obj(1).filter(|o| game.obj(*o).zone == Zone::Battlefield);
    let nonland: Vec<ObjId> = hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    if nonland.is_empty() && creature.is_none() {
        return Some(());
    }
    if !choose_use(game, d, you, "Exile a card or creature?")? {
        return Some(());
    }
    let from_hand = !nonland.is_empty() && (creature.is_none() || choose_use(game, d, you, "Choose what to exile")?);
    let (src, zcc) = cx.source;
    if from_hand {
        let rule = item_rule(game, cx.item);
        let Some(card) = choose_cards(game, d, you, &hand, &nonland, 1, 1, &rule)?.first().copied() else {
            return Some(());
        };
        if game.obj(card).zone != Zone::Hand {
            return Some(());
        }
        game.move_to(card, Zone::Exile);
        game.exiled_until_leaves.push((src, zcc, card));
        game.return_to_hand.push(card);
    } else if let Some(o) = creature {
        cx.run_on(game, d, &[EffectDef::ExileUntilSourceLeavesEffect], vec![Target::Obj(o)]);
    }
    Some(())
}
