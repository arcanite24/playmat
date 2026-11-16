//! Ugin, Eye of the Storms: "−11: Search your library for any number of colorless nonland cards, exile them, then
//! shuffle. Until end of turn, you may cast those cards without paying their mana costs."

use super::*;
use crate::card::{Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("UginEyeOfTheStorms.UginEyeOfTheStormsEffect", apply)];

/// `TargetCardInLibrary(0, MAX, colorless nonland)`; the found cards still in the library are exiled together
/// (`exileAndPlayFromExile`: cast only, without paying their mana costs, until end of turn), then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = Filter(vec![Pred::Colorless, Pred::Not { x: Box::new(Pred::Type { t: CardType::Land }) }]);
    let found = search_library(game, d, you, &filter, 0, usize::MAX, cx.source_id())?;
    let cards: Vec<ObjId> = found.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let turn = game.turn;
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            let o = game.obj_mut(c);
            o.may_play = Some((you, turn));
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.play_free = true;
            o.cast_only = true;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
