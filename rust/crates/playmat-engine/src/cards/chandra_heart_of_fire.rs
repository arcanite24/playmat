//! Chandra, Heart of Fire's −9: "Search your graveyard and library for any number of red instant and/or sorcery
//! cards, exile them, then shuffle. You may cast them this turn. Add six {R}." (the mana is its own effect)

use super::*;
use crate::card::{Color, Filter};

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ChandraHeartOfFire.ChandraHeartOfFireUltimateEffect", apply)];

/// `TargetCardInYourGraveyard(0, ∞)` (asked again after a Done with some chosen) then `TargetCardInLibrary(0, ∞)`,
/// each asked only if one can be chosen; all
/// exiled together, the library shuffled, and those still in exile may be cast this turn
/// (`PlayFromNotOwnHandZoneTargetEffect(Zone.EXILED, Duration.EndOfTurn)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let red = |game: &Game, c: ObjId| {
        let o = game.obj(c);
        (o.is(CardType::Instant) || o.is(CardType::Sorcery)) && game.colors(c).contains(&Color::R)
    };
    let mut exiled = Vec::new();
    let grave = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> = grave.iter().copied().filter(|c| red(game, *c)).collect();
    if !possible.is_empty() {
        let rule = item_rule(game, cx.item);
        exiled.extend(choose_cards(game, d, you, &grave, &possible, 0, possible.len(), &rule)?);
        // `TargetImpl.choose`: a round that ended with Done after choosing something asks again
        while !exiled.is_empty() && exiled.len() < possible.len() {
            let left: Vec<ObjId> = possible.iter().copied().filter(|c| !exiled.contains(c)).collect();
            let more = choose_cards(game, d, you, &grave, &left, 0, left.len(), &rule)?;
            if more.is_empty() {
                break;
            }
            exiled.extend(more);
        }
    }
    if game.player(you).library.iter().any(|c| red(game, *c)) {
        let filter = Filter(vec![
            Pred::Color { c: Color::R },
            Pred::Or { xs: vec![Pred::Type { t: CardType::Instant }, Pred::Type { t: CardType::Sorcery }] },
        ]);
        exiled.extend(search_library(game, d, you, &filter, 0, usize::MAX, cx.source_id())?);
    }
    for c in &exiled {
        game.move_to(*c, Zone::Exile);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    let turn = game.turn;
    for c in exiled {
        if game.obj(c).zone == Zone::Exile {
            let o = game.obj_mut(c);
            o.may_play = Some((you, turn));
            o.may_play_end_step = false;
            o.may_play_while = None;
        }
    }
    Some(())
}
