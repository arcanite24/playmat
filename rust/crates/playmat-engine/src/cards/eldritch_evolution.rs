//! Eldritch Evolution: "As an additional cost to cast this spell, sacrifice a creature. Search your library for a
//! creature card with mana value X or less, where X is 2 plus the sacrificed creature's mana value. Put that card
//! onto the battlefield, then shuffle. Exile Eldritch Evolution."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EldritchEvolution.EldritchEvolutionEffect", apply)];

/// The sacrificed creature's mana value (`SacrificeTargetCost.getPermanents`), one creature card of at most 2 more
/// (`TargetCardInLibrary(filter)`: one), onto the battlefield, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.item.sacrificed_stats[2] + 2;
    let filter = Filter(vec![Pred::Mv { cmp: Cmp::FewerThan, n: n + 1 }, Pred::Type { t: CardType::Creature }]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&c) = found.first() {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
