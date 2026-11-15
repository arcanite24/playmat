//! Velomachus Lorehold: "Flying, vigilance, haste. Whenever Velomachus Lorehold attacks, look at the top seven cards
//! of your library. You may cast an instant or sorcery spell with mana value less than or equal to Velomachus
//! Lorehold's power from among them without paying its mana cost. Put the rest on the bottom of your library in a
//! random order."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("VelomachusLorehold.VelomachusLoreholdEffect", apply)];

/// The top seven (only looked at, no window), `castSpellWithAttributesForFree` among them with an instant or sorcery
/// filter of mana value less than its power plus one (as it is, or as it last was), then those still in the library
/// to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let power = if game.obj(me).zone == Zone::Battlefield {
        game.power_toughness(me).0
    } else {
        game.obj(me).power_lki.unwrap_or(0)
    };
    let cards: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    let filter = Filter::of(vec![
        Pred::Or { xs: vec![Pred::Type { t: CardType::Instant }, Pred::Type { t: CardType::Sorcery }] },
        Pred::Mv { cmp: Cmp::FewerThan, n: power + 1 },
    ]);
    if !cast_one_for_free_from(game, d, you, me, &filter, &cards) {
        return None;
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false).then_some(())
}
