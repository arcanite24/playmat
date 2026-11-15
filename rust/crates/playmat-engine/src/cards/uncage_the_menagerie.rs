//! Uncage the Menagerie: "Search your library for up to X creature cards with different names that each have mana
//! value X, reveal them, put them into your hand, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UncageTheMenagerie.UncageTheMenagerieEffect", apply)];

/// `SearchLibraryPutInHandEffect(TargetCardWithDifferentNameInLibrary(0, X, creature card with mana value X), true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x.max(0);
    if x == 0 {
        // up to zero cards: Java's `TargetCardWithDifferentNameInLibrary(0, 0, …)` asks nothing, then it shuffles
        let you = cx.controller();
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
        return Some(());
    }
    let filter = Filter(vec![
        Pred::Type { t: CardType::Creature },
        Pred::Mv { cmp: Cmp::EqualTo, n: x },
        Pred::DifferentName,
    ]);
    let e = EffectDef::SearchLibraryEffect {
        filter,
        min: 0,
        max: x as usize,
        to: crate::card::PutCards::Hand,
        optional: false,
        target_controller: false,
    };
    cx.run(game, d, &[e]);
    Some(())
}
