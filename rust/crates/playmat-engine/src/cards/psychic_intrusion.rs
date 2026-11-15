//! Psychic Intrusion: "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or
//! hand and exile it. You may cast that card for as long as it remains exiled, and you may spend mana as though it
//! were mana of any color to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PsychicIntrusion.PsychicIntrusionExileEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])];

/// The hand revealed; with nonland cards in both zones, "Exile card from opponents Hand?" (yes: the hand), else the
/// zone that has one; the card chosen (`TargetCard`) is exiled, castable (only cast) by you for as long as it stays
/// there, with mana of any color
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = cx.player(0)?;
    let src = cx.source_id();
    let hand = game.player(p).hand.clone();
    let zcc = game.obj(src).zcc;
    let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards });
    let nonland =
        |game: &Game, zone: &[ObjId]| -> Vec<ObjId> {
            zone.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(src), *c)).collect()
        };
    let graveyard = game.player(p).graveyard.clone();
    let in_hand = nonland(game, &hand);
    let in_graveyard = nonland(game, &graveyard);
    let from_hand = if !in_hand.is_empty() && !in_graveyard.is_empty() {
        choose_use(game, d, you, "Exile card from opponents Hand?")?
    } else {
        !in_hand.is_empty()
    };
    let (shown, possible) = if from_hand { (hand, in_hand) } else { (graveyard, in_graveyard) };
    if possible.is_empty() {
        return Some(());
    }
    let Some(&card) = choose_cards(game, d, you, &shown, &possible, 1, 1, "")?.first() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let o = game.obj_mut(card);
        o.may_play = Some((you, u32::MAX));
        o.may_play_end_step = false;
        o.may_play_while = None;
        o.cast_only = true;
        o.any_mana = true;
    }
    Some(())
}
