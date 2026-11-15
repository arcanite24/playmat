//! Elspeth Resplendent: "+1: Choose up to one target creature. Put a +1/+1 counter and a counter from among flying,
//! first strike, lifelink, or vigilance on it. −3: Look at the top seven cards of your library. You may put a
//! permanent card with mana value 3 or less from among them onto the battlefield with a shield counter on it. Put the
//! rest on the bottom of your library in a random order. −7: Create five 3/3 white Angel creature tokens with
//! flying."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ElspethResplendent.ElspethResplendentCounterEffect", counters),
    Handler::effect("ElspethResplendent.ElspethResplendentLookEffect", look),
];

const CHOICES: [&str; 4] = ["Flying", "First strike", "Lifelink", "Vigilance"];

const PERMANENT_TYPES: [CardType; 6] = [
    CardType::Artifact,
    CardType::Battle,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Land,
    CardType::Planeswalker,
];

/// A +1/+1 counter on the target, then "Choose an ability counter to add" (required) and that counter
fn counters(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    add_counters_by(game, o, "+1/+1", 1, Some(you));
    let choices: Vec<String> = CHOICES.iter().map(|c| c.to_string()).collect();
    let q = Query::Choice { message: "Choose an ability counter to add".into(), choices: choices.clone() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) if choices.contains(&t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if game.obj(o).zone == Zone::Battlefield {
        add_counters_by(game, o, &chosen.to_lowercase(), 1, Some(you));
    }
    Some(())
}

/// The top seven: up to one permanent card with mana value 3 or less among them (`TargetCardInLibrary(0, 1)`) onto
/// the battlefield with a shield counter; those still in the library to the bottom in a random order
fn look(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    let possible: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            PERMANENT_TYPES.into_iter().any(|t| o.is(t))
                && o.def.mana_value() < 4
        })
        .collect();
    let chosen = choose_cards(game, d, you, &cards, &possible, 0, 1, "permanent card with mana value 3 or less")?;
    if let Some(&card) = chosen.first() {
        put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield);
        if d.stopped() {
            return None;
        }
        if game.obj(card).zone == Zone::Battlefield {
            add_counters_by(game, card, "shield", 1, Some(you));
        }
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false);
    (!d.stopped()).then_some(())
}
