//! Instrument of the Bards: "At the beginning of your upkeep, you may put a harmony counter on Instrument of the
//! Bards. {3}{G}, {T}: Search your library for a creature card with mana value equal to the number of harmony
//! counters on Instrument of the Bards, reveal it, and put it into your hand. If that card is legendary, create a
//! Treasure token. Then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("InstrumentOfTheBards.InstrumentOfTheBardsEffect", apply).tokens(&["TreasureToken"])];

/// the harmony counters on it (as it last was), one creature card of that mana value revealed and put into your hand,
/// a Treasure if it's legendary, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let s = game.obj(cx.source_id());
    let counters = if s.zone == Zone::Battlefield && s.zcc == cx.source.1 {
        s.counter("harmony")
    } else {
        s.lki_counters.iter().filter(|(k, _)| k == "harmony").map(|(_, n)| *n).sum()
    };
    let filter = Filter(vec![
        crate::card::Pred::Type { t: CardType::Creature },
        crate::card::Pred::Mv { cmp: crate::card::Cmp::EqualTo, n: counters },
    ]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc: cx.source.1,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        game.move_to(card, Zone::Hand);
        if game.obj(card).def.supertypes.iter().any(|s| s == "LEGENDARY") {
            cx.create_tokens(game, d, &cx.tokens[0], 1);
        }
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
