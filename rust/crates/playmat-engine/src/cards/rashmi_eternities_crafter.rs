//! Rashmi, Eternities Crafter: "Whenever you cast your first spell each turn, reveal the top card of your library.
//! You may cast it without paying its mana cost if it's a spell with lesser mana value. If you don't cast it, put it
//! into your hand."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "RashmiEternitiesCrafter.RashmiEternitiesCrafterTriggeredAbility",
        "new SpellCastControllerTriggeredAbility(new RashmiEternitiesCrafterEffect(), false)",
        first_spell,
    ),
    Handler::effect("RashmiEternitiesCrafter.RashmiEternitiesCrafterEffect", apply),
];

/// `SpellsCastWatcher.getCount(player) == 1`: the spell this trigger saw is the first you cast this turn
fn first_spell(game: &Game, item: &StackItem) -> bool {
    let Some(spell) = item.cast_spell else { return false };
    game.watch.spells.iter().find(|s| s.caster == item.controller).map(|s| s.card) == Some(spell)
}

/// The top card revealed, `castSpellWithAttributesForFree` of it with mana value less than the spell's (the
/// trigger's X: its mana value as cast); still in the library, it goes to the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.item.cast_spell?;
    let card = game.player(you).library.first().copied()?;
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    let filter = Filter::of(vec![Pred::Mv { cmp: Cmp::FewerThan, n: cx.item.x }]);
    if !cast_one_for_free_from(game, d, you, cx.source_id(), &filter, &[card]) {
        return None;
    }
    if game.obj(card).zone == Zone::Library {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}
