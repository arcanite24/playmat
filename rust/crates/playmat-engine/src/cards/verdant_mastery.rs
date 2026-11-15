//! Verdant Mastery: "You may pay {3}{G} rather than pay this spell's mana cost. Search your library for up to four
//! basic land cards and reveal them. Put one of them onto the battlefield tapped under an opponent's control if the
//! {3}{G} cost was paid. Put two of them onto the battlefield tapped under your control and the rest into your hand.
//! Then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VerdantMastery.VerdantMasteryEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// The search (`TargetCardInLibrary(0, 4, …)`) and the reveal; with nothing found, only the shuffle. Cast for the
/// alternative cost: an opponent you choose (`chooseTarget` of a not-targeted `TargetOpponent`) and one of the found
/// cards you choose put onto the battlefield tapped under their control. Then two of those still in the library (or
/// all, with fewer) onto the battlefield tapped under yours, the rest into your hand, and the shuffle.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 0, 4, cx.source_id())?;
    if !found.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = found.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        let rule = item_rule(game, cx.item);
        if game.obj(cx.source_id()).alternative_paid {
            let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
            if !opponents.is_empty() {
                let opponent = loop {
                    let q = Query::Target { players: opponents.clone(), objs: Vec::new(), required: true };
                    match ask(game, d, you, &q) {
                        Answer::Player(p) if opponents.contains(&p) => break p,
                        Answer::Stop => return None,
                        _ => {}
                    }
                    if d.stopped() {
                        return None;
                    }
                };
                let given = choose_cards(game, d, you, &found, &found, 1, 1, &rule)?;
                if !given.is_empty() && !put_cards(game, d, opponent, &given, crate::card::PutCards::BattlefieldTapped)
                {
                    return None;
                }
            }
        }
        let left: Vec<ObjId> = found.iter().copied().filter(|c| game.obj(*c).zone == Zone::Library).collect();
        if !left.is_empty() {
            let n = left.len().min(2);
            let chosen = choose_cards(game, d, you, &left, &left, n, n, &rule)?;
            if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
                return None;
            }
            let rest: Vec<ObjId> = left.iter().copied().filter(|c| game.obj(*c).zone == Zone::Library).collect();
            put_cards(game, d, you, &rest, crate::card::PutCards::Hand);
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
