//! Prismatic Undercurrents: "Vivid — When this enchantment enters, search your library for up to X basic land cards,
//! where X is the number of colors among permanents you control. Reveal those cards, put them into your hand, then
//! shuffle. You may play an additional land on each of your turns."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PrismaticUndercurrents.PrismaticUndercurrentsEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LANDS"])];

/// X the colors among permanents you control (`ColorsAmongControlledPermanentsCount.ALL_PERMANENTS`), the search for
/// up to X basic lands, revealed, to hand, and the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut colors: Vec<crate::card::Color> = Vec::new();
    for b in game.battlefield.iter() {
        if game.obj(*b).controller == you {
            for c in game.colors(*b) {
                if !colors.contains(&c) {
                    colors.push(c);
                }
            }
        }
    }
    let found = search_library(game, d, you, &cx.filters[0], 0, colors.len(), cx.source_id())?;
    let found: Vec<ObjId> = found.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !found.is_empty() {
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc: cx.source.1,
            paren: false,
            cards: found.iter().map(|c| (*c, Zone::Library)).collect(),
        });
    }
    for c in found {
        game.move_to(c, Zone::Hand);
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
