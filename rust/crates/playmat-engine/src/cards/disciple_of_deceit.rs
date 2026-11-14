//! Disciple of Deceit: "Inspired — Whenever Disciple of Deceit becomes untapped, you may discard a nonland card. If
//! you do, search your library for a card with the same mana value as that card, reveal it, put it into your hand,
//! then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiscipleOfDeceit.DiscipleOfDeceitEffect", apply)];

/// with a nonland card in hand: `chooseUse`, the `DiscardTargetCost` (cancellable), then the search for its mana
/// value, revealed, to hand, and the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let nonland: Vec<ObjId> = hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    if nonland.is_empty() || !choose_use(game, d, you, "Discard a nonland card to search your library?")? {
        return Some(());
    }
    let chosen = choose_cards_with(game, d, you, &hand, &nonland, 1, 1, "discard", true)?;
    let Some(&c) = chosen.first() else { return Some(()) };
    discard_card(game, you, c);
    game.discarded_cards();
    let mv = game.obj(c).def.mana_value() as i32;
    let filter = Filter(vec![crate::card::Pred::Mv { cmp: crate::card::Cmp::EqualTo, n: mv }]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        let zcc = cx.source.1;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        game.move_to(card, Zone::Hand);
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
