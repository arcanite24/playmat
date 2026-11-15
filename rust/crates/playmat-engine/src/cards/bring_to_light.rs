//! Bring to Light: "Converge — Search your library for a creature, instant, or sorcery card with mana value less than
//! or equal to the number of colors of mana spent to cast this spell, exile that card, then shuffle. You may cast
//! that card without paying its mana cost."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("BringToLight.BringToLightEffect", apply)];

/// `ColorsOfManaSpentToCastCount` as the mana value limit; the search (one card), the exile, the shuffle, then
/// `castSpellWithAttributesForFree` of what was found
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.obj(cx.source_id()).spent_colors.iter().filter(|n| **n > 0).count() as i32;
    let types = [CardType::Creature, CardType::Instant, CardType::Sorcery];
    let filter = Filter::of(vec![
        Pred::Or { xs: types.iter().map(|t| Pred::Type { t: *t }).collect() },
        Pred::Mv { cmp: Cmp::FewerThan, n: n + 1 },
    ]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    let card = found.first().copied();
    if let Some(c) = card {
        game.move_to(c, Zone::Exile);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    if let Some(c) = card {
        super::breaching_dragonstorm::cast_for_free(game, d, you, c, |_, _| true)?;
    }
    Some(())
}
