//! Neoform: "Search your library for a creature card with mana value equal to 1 plus the sacrificed creature's
//! mana value, put that card onto the battlefield with an additional +1/+1 counter on it, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Neoform.NeoformEffect", apply)];

/// The sacrificed creature's mana value (`SacrificeTargetCost.getPermanents`) plus one; the card found enters with
/// a +1/+1 counter (`setEnterWithCounters`); then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.item.sacrificed_stats[2] + 1;
    let filter = Filter(vec![Pred::Mv { cmp: Cmp::EqualTo, n }, Pred::Type { t: CardType::Creature }]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&c) = found.first() {
        if game.obj(c).zone == Zone::Library {
            enter_battlefield(game, d, c, you, false);
            if game.obj(c).zone == Zone::Battlefield {
                add_counters_by(game, c, "+1/+1", 1, Some(you));
            }
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
