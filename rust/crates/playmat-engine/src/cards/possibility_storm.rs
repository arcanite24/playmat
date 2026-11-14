//! Possibility Storm: "Whenever a player casts a spell from their hand, that player exiles it, then exiles cards from
//! the top of their library until they exile a card that shares a card type with it. That player may cast that card
//! without paying its mana cost. Then they put all cards exiled with Possibility Storm on the bottom of their
//! library in a random order."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "PossibilityStorm.PossibilityStormTriggeredAbility",
        "new SpellCastAllTriggeredAbility(new PossibilityStormEffect(), StaticFilters.FILTER_SPELL_A, false)",
        from_hand,
    ),
    Handler::effect("PossibilityStorm.PossibilityStormEffect", apply),
];

/// the spell was cast from a hand (`event.getZone() == Zone.HAND`), and is still a spell
fn from_hand(game: &Game, item: &StackItem) -> bool {
    item.cast_spell.is_some_and(|s| game.obj(s).zone == Zone::Stack && game.obj(s).cast_from == Some(Zone::Hand))
}

/// The spell exiled (already gone from the stack: as it last was); its controller exiles from the top until a card
/// sharing a card type with it (or the library runs out); that card, a nonland one with targets it can choose, may
/// be cast free; then every card in Possibility Storm's exile goes to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let on_stack = game.obj(spell).zone == Zone::Stack;
    let caster = game
        .stack
        .iter()
        .rev()
        .find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none())
        .map_or(game.obj(spell).controller, |i| i.controller);
    let types: Vec<CardType> = game.obj(spell).def.types.clone();
    let source = cx.source_id();
    let exile = |game: &mut Game, c: ObjId| -> bool {
        game.move_to(c, Zone::Exile);
        if game.obj(c).zone != Zone::Exile {
            return false;
        }
        let zcc = game.obj(source).zcc;
        game.obj_mut(c).exiled_with = Some((source, zcc));
        true
    };
    if on_stack {
        Arc::make_mut(&mut game.stack).retain(|i| i.card != spell || i.ability.is_some() || i.delayed.is_some());
        exile(game, spell);
    }
    if game.player(caster).library.is_empty() {
        return Some(());
    }
    let shares = |game: &Game, c: ObjId| game.obj(c).def.types.iter().any(|t| types.contains(t));
    let mut card = None;
    while let Some(top) = game.player(caster).library.first().copied() {
        card = Some(top);
        if !exile(game, top) {
            break;
        }
        if game.player(caster).library.is_empty() || shares(game, top) {
            break;
        }
    }
    if let Some(c) = card {
        let def = game.obj(c).def.clone();
        if shares(game, c) && !game.obj(c).is(CardType::Land) && has_targets(game, caster, c, &def) {
            let message = format!("Cast {} without paying cost?", log::object(game, c));
            if choose_use(game, d, caster, &message)? {
                cast_free(game, d, caster, c);
                if d.stopped() {
                    return None;
                }
            }
        }
    }
    // `getExileZone(source.getSourceId())`: everything exiled with it, by any of its triggers
    let exiled: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| game.obj(*c).exiled_with.is_some_and(|(s, _)| s == source))
        .collect();
    if !exiled.is_empty() && !put_cards(game, d, caster, &exiled, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
