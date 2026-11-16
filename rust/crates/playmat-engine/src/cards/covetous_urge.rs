//! Covetous Urge: "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand
//! and exile it. You may cast that card for as long as it remains exiled, and you may spend mana as though it were
//! mana of any color to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("CovetousUrge.CovetousUrgeEffect", apply).filters(&["StaticFilters.FILTER_CARD_A_NON_LAND"])];

/// The hand revealed; an empty graveyard or a yes to "Exile a nonland card from <player>'s hand or graveyard"
/// (Hand / Graveyard) picks the hand (nothing more with an empty hand), else the graveyard; the nonland card chosen
/// there (`TargetCard`) is exiled, castable (only cast) by you for as long as it stays there, with mana of any color
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = cx.player(0)?;
    let src = cx.source_id();
    let hand = game.player(p).hand.clone();
    let zcc = game.obj(src).zcc;
    let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards });
    let graveyard = game.player(p).graveyard.clone();
    let from_hand = graveyard.is_empty() || {
        let message = format!("Exile a nonland card from {}'s hand or graveyard", log::seat_name(p));
        choose_use(game, d, you, &message)?
    };
    let shown = if from_hand { hand } else { graveyard };
    if shown.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> =
        shown.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(src), *c)).collect();
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &shown, &possible, 1, 1, &rule)?.first() else { return Some(()) };
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
