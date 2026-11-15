//! Deadly Cover-Up: "As an additional cost to cast this spell, you may collect evidence 6. Destroy all creatures. If
//! evidence was collected, exile a card from an opponent's graveyard. Then search its owner's graveyard, hand, and
//! library for any number of cards with that name and exile them. That player shuffles, then draws a card for each
//! card exiled from their hand this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeadlyCoverUp.DeadlyCoverUpEffect", apply)];

/// `TargetCardInOpponentsGraveyard` (not targeted) chosen and exiled, then `applySearchAndExile` with its name and
/// owner: the graveyard's cards of the name optional, then the hand's and the library's, the shuffle and the draws
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let graveyards: Vec<ObjId> = (0..game.seats())
        .filter(|p| *p != you && !game.player(*p).lost)
        .flat_map(|p| game.player(p).graveyard.clone())
        .collect();
    if graveyards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &graveyards, &graveyards, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    let name = game.obj(card).def.name.clone();
    let p = game.obj(card).owner;
    game.move_to(card, Zone::Exile);
    let named = |game: &Game, zone: &[ObjId]| -> Vec<ObjId> {
        zone.iter().copied().filter(|c| game.obj(*c).def.name == name).collect()
    };
    // the graveyard: asked only when a card of the name is there
    let grave = game.player(p).graveyard.clone();
    let in_grave = named(game, &grave);
    if !in_grave.is_empty() {
        let n = in_grave.len();
        for c in choose_cards(game, d, you, &grave, &in_grave, 0, n, "exile")? {
            game.move_to(c, Zone::Exile);
        }
    }
    // the hand
    let hand = game.player(p).hand.clone();
    let in_hand = named(game, &hand);
    let mut from_hand = 0;
    if !hand.is_empty() && !in_hand.is_empty() {
        let n = in_hand.len();
        let chosen = choose_cards(game, d, you, &hand, &in_hand, 0, n, "exile")?;
        from_hand = chosen.len();
        for c in chosen {
            game.move_to(c, Zone::Exile);
        }
    }
    // the library
    let library = game.player(p).library.clone();
    let in_library = named(game, &library);
    if !library.is_empty() && !in_library.is_empty() {
        let n = in_library.len();
        for c in choose_cards(game, d, you, &library, &in_library, 0, n, "exile")? {
            game.move_to(c, Zone::Exile);
        }
    }
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
    if from_hand > 0 {
        draw(game, p, from_hand);
    }
    Some(())
}
