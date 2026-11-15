//! Oswald Fiddlebender: "Magical Tinkering — {W}, {T}, Sacrifice an artifact: Search your library for an artifact
//! card with mana value equal to 1 plus the sacrificed artifact's mana value, put it onto the battlefield, then
//! shuffle. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OswaldFiddlebender.OswaldFiddlebenderEffect", apply)];

/// the sacrificed artifact's mana value plus one: one artifact card of it onto the battlefield, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if cx.item.sacrificed.is_empty() {
        return Some(());
    }
    let n = cx.item.sacrificed_stats[2] + 1;
    let filter = Filter(vec![
        crate::card::Pred::Type { t: CardType::Artifact },
        crate::card::Pred::Mv { cmp: crate::card::Cmp::EqualTo, n },
    ]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&c) = found.first() {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
