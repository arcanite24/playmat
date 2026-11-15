//! Genesis Hydra: "When you cast this spell, reveal the top X cards of your library. You may put a nonland permanent
//! card with mana value X or less from among them onto the battlefield. Then shuffle the rest into your library.
//! Genesis Hydra enters with X +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("GenesisHydra.GenesisHydraPutOntoBattlefieldEffect", apply)];

/// X is the cast spell's (`SOURCE_CAST_SPELL_ABILITY`'s X tag); with X 0 nothing. The top X revealed; with a
/// nonland permanent card of mana value X or less among them, up to one chosen (`TargetCard(0, 1)`) onto the
/// battlefield; then, with any left, the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let x = game.stack.iter().find(|i| i.card == spell && i.ability.is_none()).map_or(0, |i| i.x);
    if x <= 0 {
        return Some(());
    }
    let mut cards: Vec<ObjId> = game.player(you).library.iter().take(x as usize).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    let possible: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.def.is_permanent_card() && !o.is(CardType::Land) && o.def.mana_value() as i32 <= x
        })
        .collect();
    if !possible.is_empty() {
        let rule = item_rule(game, cx.item);
        if let Some(c) = choose_cards(game, d, you, &cards, &possible, 0, 1, &rule)?.first().copied() {
            cards.retain(|x| *x != c);
            put_onto_battlefield(game, d, c, you);
            if d.stopped() {
                return None;
            }
        }
    }
    if !cards.is_empty() {
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    Some(())
}
