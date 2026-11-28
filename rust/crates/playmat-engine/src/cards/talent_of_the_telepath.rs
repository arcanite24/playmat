//! Talent of the Telepath: "Target opponent reveals the top seven cards of their library. You may cast an instant
//! or sorcery spell from among them without paying its mana cost. Then that player puts the rest into their
//! graveyard. Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, you may cast
//! up to two instant and/or sorcery spells from among the revealed cards instead of one."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TalentOfTheTelepath.TalentOfTheTelepathEffect", apply)];

fn instant_or_sorcery(game: &Game, c: ObjId) -> bool {
    game.has_type(c, CardType::Instant) || game.has_type(c, CardType::Sorcery)
}

/// The top seven revealed; `castMultipleWithAttributeForFree` of instant and sorcery cards, at most two with spell
/// mastery (else one: `castSpellWithAttributesForFree` alone); what's still in the library into the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = cx.player(0)?;
    let top: Vec<ObjId> = game.player(p).library.iter().take(7).copied().collect();
    let mastery = game.player(you).graveyard.iter().filter(|c| instant_or_sorcery(game, **c)).count() >= 2;
    cast_up_to(game, d, cx, you, top.clone(), if mastery { 2 } else { 1 })?;
    for c in top {
        if game.obj(c).zone == Zone::Library {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}

/// `castSpellWithAttributesForFree`: one castable card is taken as it is, more are chosen among (up to one), then
/// "Cast spell without paying its mana cost (…)?". Whether one was cast.
fn cast_one(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, you: Seat, options: &[ObjId]) -> Option<bool> {
    let card = match options.len() {
        0 => None,
        1 => Some(options[0]),
        _ => {
            let rule = item_rule(game, cx.item);
            choose_cards(game, d, you, options, options, 0, 1, &rule)?.first().copied()
        }
    };
    let Some(card) = card else { return Some(false) };
    let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
    if !choose_use(game, d, you, &message)? {
        return Some(false);
    }
    let cast = cast_free(game, d, you, card);
    if d.stopped() {
        return None;
    }
    Some(cast)
}

/// `castMultipleWithAttributeForFree(…, maxSpells)`: up to `most` casts; a cast not made asks whether to go on
fn cast_up_to(
    game: &mut Game,
    d: &mut dyn Decider,
    cx: &EffectCx,
    you: Seat,
    mut cards: Vec<ObjId>,
    most: usize,
) -> Option<()> {
    let castable = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards.iter().copied().filter(|c| instant_or_sorcery(game, *c)).collect()
    };
    if most == 1 {
        let options = castable(game, &cards);
        cast_one(game, d, cx, you, &options)?;
        return Some(());
    }
    let max = cards.len().min(most);
    let mut cast_count = 0;
    cards.retain(|c| game.obj(*c).zone != Zone::Stack);
    if castable(game, &cards).is_empty() {
        return Some(());
    }
    loop {
        let options = castable(game, &cards);
        let cast = cast_one(game, d, cx, you, &options)?;
        cards.retain(|c| game.obj(*c).zone != Zone::Stack);
        if cards.is_empty() || castable(game, &cards).is_empty() {
            break;
        }
        if cast {
            cast_count += 1;
            if cast_count >= max {
                break;
            }
        } else {
            let message = format!("You can cast {} more spells for free. Continue casting?", max - cast_count);
            if !choose_use(game, d, you, &message)? {
                break;
            }
        }
    }
    Some(())
}
