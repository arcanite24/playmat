//! Corpse Appraiser: "When Corpse Appraiser enters, exile up to one target creature card from a graveyard. If a card
//! is put into exile this way, look at the top three cards of your library, then put one of those cards into your
//! hand and the rest into your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CorpseAppraiser.CorpseAppraiserEffect", apply)];

/// `player.choose(cards, TargetCardInLibrary())`: one card to hand, the rest to the graveyard together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    game.move_to(c, Zone::Exile);
    if game.obj(c).zone != Zone::Exile {
        return Some(());
    }
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &top, 1, 1, &rule)?;
    if let Some(h) = chosen.first() {
        game.move_to(*h, Zone::Hand);
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    for c in rest {
        game.move_to(c, Zone::Graveyard);
    }
    Some(())
}
