//! Choco, Seeker of Paradise: "Whenever one or more Birds you control attack, look at that many cards from the top
//! of your library. You may put one of them into your hand. Then put any number of land cards from among them onto
//! the battlefield tapped and the rest into your graveyard. Landfall — Whenever a land you control enters, Choco gets
//! +1/+0 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChocoSeekerOfParadise.ChocoSeekerOfParadiseEffect", apply)
    .filters(&["filter", "StaticFilters.FILTER_CARD_LANDS"])];

/// "That many": the attacking Birds you control (`VALUEKEY_NUMBER_ATTACKERS`, counted as it resolves); up to one of
/// those cards to your hand (`player.choose`), then any number of the lands left onto the battlefield tapped, the
/// rest into your graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).attacking.is_some() && game.filter_matches(&cx.filters[0], you, Some(src), **o))
        .count();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(n).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let to_hand = super::gilgamesh_master_at_arms::choose_any(game, d, you, &cards, 1)?;
    if let Some(&c) = to_hand.first() {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Hand) {
            return None;
        }
    }
    let left: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    let lands: Vec<ObjId> =
        left.iter().copied().filter(|c| game.filter_matches(&cx.filters[1], you, Some(src), *c)).collect();
    // (`HumanPlayer.choose` over the cards left: asked even with no land among them)
    let chosen =
        if left.is_empty() { Vec::new() } else { super::gilgamesh_master_at_arms::choose_any(game, d, you, &lands, usize::MAX)? };
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !rest.is_empty() && !put_cards(game, d, you, &rest, crate::card::PutCards::Graveyard) {
        return None;
    }
    Some(())
}
