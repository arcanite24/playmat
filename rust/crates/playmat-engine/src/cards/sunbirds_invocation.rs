//! Sunbird's Invocation: "Whenever you cast a spell from your hand, reveal the top X cards of your library, where X
//! is that spell's mana value. You may cast a spell with mana value X or less from among cards revealed this way
//! without paying its mana cost. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SunbirdsInvocation.SunbirdsInvocationEffect", apply)];

/// X: the spell's mana value (its X included while it's on the stack); the top X revealed, then
/// `CardUtil.castSpellWithAttributesForFree` over them with "mana value X or less" (one is taken as it is, more
/// are chosen among, up to one; `chooseUse`, the free cast), and the rest still in the library to the bottom in a
/// random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(spell) = cx.item.cast_spell.or_else(|| cx.obj(0)) else { return Some(()) };
    let x = game.stack.iter().rev().find(|i| i.card == spell && i.ability.is_none()).map_or(0, |i| i.x);
    let mv = {
        let c = game.obj(spell).def.printed();
        c.mana_value() as i32 + x * c.x as i32
    };
    let cards: Vec<ObjId> = game.player(you).library.iter().take(mv.max(0) as usize).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let (src, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: shown });
    let castable: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| !game.obj(*c).is(CardType::Land) && game.obj(*c).def.mana_value() as i32 <= mv)
        .collect();
    let card = match castable.len() {
        0 => None,
        1 => Some(castable[0]),
        _ => {
            let rule = item_rule(game, cx.item);
            choose_cards(game, d, you, &castable, &castable, 0, 1, &rule)?.first().copied()
        }
    };
    if let Some(card) = card {
        super::breaching_dragonstorm::cast_for_free(game, d, you, card, |_, _| true)?;
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
