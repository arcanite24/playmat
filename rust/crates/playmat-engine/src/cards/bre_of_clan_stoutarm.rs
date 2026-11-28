//! Bre of Clan Stoutarm: "{1}{W}, {T}: Another target creature you control gains flying and lifelink until end of
//! turn. At the beginning of your end step, if you gained life this turn, exile cards from the top of your library
//! until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is
//! less than or equal to the amount of life you gained this turn. Otherwise, put it into your hand."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("BreOfClanStoutarm.BreOfClanStoutarmEffect", apply)];

/// Cards exiled from the top one at a time until a nonland one; `castSpellWithAttributesForFree` of it with mana
/// value at most the life you gained this turn, else (still in exile) it goes to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut found = None;
    while let Some(&c) = game.player(you).library.first() {
        game.move_to(c, Zone::Exile);
        if game.obj(c).zone != Zone::Exile {
            break;
        }
        if !game.obj(c).is(CardType::Land) {
            found = Some(c);
            break;
        }
    }
    let card = found?;
    let gained = game.player(you).life_gained;
    let filter = Filter::of(vec![Pred::Mv { cmp: Cmp::OrLess, n: gained }]);
    if !cast_one_for_free_from(game, d, you, cx.source_id(), &filter, &[card]) {
        return None;
    }
    if game.obj(card).zone == Zone::Exile {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}
