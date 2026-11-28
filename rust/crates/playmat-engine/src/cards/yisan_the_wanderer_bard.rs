//! Yisan, the Wanderer Bard: "Search your library for a creature card with mana value equal to the number of verse
//! counters on Yisan, put it onto the battlefield, then shuffle."

use super::*;
use crate::card::{Cmp, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("YisanTheWandererBard.YisanTheWandererBardEffect", apply)];

/// The counters on Yisan now, or as it last was on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = game.obj(cx.source_id());
    let n = if cx.source_on_battlefield(game) {
        src.counter("verse")
    } else {
        src.lki_counters.iter().filter(|(c, _)| c == "verse").map(|(_, k)| *k).sum()
    };
    let filter = Filter::of(vec![Pred::Type { t: CardType::Creature }, Pred::Mv { cmp: Cmp::EqualTo, n }]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield);
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
