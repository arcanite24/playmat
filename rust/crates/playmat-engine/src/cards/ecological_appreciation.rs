//! Ecological Appreciation: "Search your library and graveyard for up to four creature cards with different names
//! that each have mana value X or less and reveal them. An opponent chooses two of those cards. Shuffle the chosen
//! cards into your library and put the rest onto the battlefield. Exile Ecological Appreciation."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("EcologicalAppreciation.EcologicalAppreciationEffect", apply)];

/// `TargetCardWithDifferentNameInLibrary(0, 4, creature card with mana value X or less)`; then, fewer than four
/// found, up to the rest from your graveyard (`TargetCardInYourGraveyard`, each with a name none of the others
/// has); the one opponent chooses two of them (`TargetCard(2, ALL)`), put on top of your library, the rest onto the
/// battlefield; then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0);
    let filter = Filter::of(vec![Pred::Type { t: CardType::Creature }, Pred::Mv { cmp: Cmp::FewerThan, n: x + 1 }]);
    let mut library_filter = filter.clone();
    library_filter.0.push(Pred::DifferentName);
    let found = search_library(game, d, you, &library_filter, 0, 4, cx.source_id())?;
    let mut cards: Vec<ObjId> = found.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    let remaining = 4 - cards.len();
    if remaining > 0 {
        let from_library = cards.clone();
        let mut chosen: Vec<ObjId> = Vec::new();
        loop {
            if chosen.len() >= remaining {
                break;
            }
            let left: Vec<ObjId> = game
                .player(you)
                .graveyard
                .iter()
                .copied()
                .filter(|c| !chosen.contains(c) && game.filter_matches(&filter, you, Some(cx.source_id()), *c))
                .filter(|c| {
                    let name = game.obj(*c).name();
                    from_library.iter().chain(chosen.iter()).all(|o| game.obj(*o).name() != name)
                })
                .collect();
            if left.is_empty() {
                break;
            }
            match ask(game, d, you, &Query::Cards { objs: left.clone(), required: false }) {
                Answer::Obj(o) if left.contains(&o) => chosen.push(o),
                Answer::Stop => return None,
                Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                _ => break,
            }
            if d.stopped() {
                return None;
            }
        }
        cards.extend(chosen);
        cards.retain(|c| matches!(game.obj(*c).zone, Zone::Graveyard | Zone::Library));
    }
    // (`player.choose(TargetOpponent)`: `HumanPlayer.choose(Target)` asks even with one opponent)
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    if opponents.is_empty() {
        return Some(());
    }
    let opponent = loop {
        match ask(game, d, you, &Query::Target { players: opponents.clone(), objs: Vec::new(), required: true }) {
            Answer::Player(p) if opponents.contains(&p) => break p,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let n = cards.len().min(2);
    let to_shuffle = choose_cards(game, d, opponent, &cards, &cards, n, n, &item_rule(game, cx.item))?;
    for c in &to_shuffle {
        game.move_to(*c, Zone::Library);
    }
    let rest: Vec<ObjId> = cards.iter().copied().filter(|c| !to_shuffle.contains(c)).collect();
    if !rest.is_empty() && !put_cards(game, d, you, &rest, crate::card::PutCards::Battlefield) {
        return None;
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
