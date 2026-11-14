//! Wandering Archaic: "Whenever an opponent casts an instant or sorcery spell, they may pay {2}. If they don't, you
//! may copy that spell. You may choose new targets for the copy." Its back, Explore the Vastlands: "Each player looks
//! at the top five cards of their library, reveals a land card and/or an instant or sorcery card from among them,
//! then puts the cards they revealed this way into their hand and the rest on the bottom of their library in a random
//! order. Each player gains 3 life."

use super::*;
use crate::card::CostDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("WanderingArchaic.WanderingArchaicEffect", apply),
    Handler::effect("WanderingArchaic.ExploreTheVastlandsEffect", explore),
];

/// the opponent the trigger points at (`SetTargetPointer.PLAYER`) and the spell (`getValue("spellCast")`): `canPay`
/// (a mana cost always can), "Pay {2}?", the payment; else "Copy <spell>?" and `createCopyOnStack` with new targets
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    let cost = [CostDef::ManaCost { cost: "{2}".into() }];
    if pay_unless(game, d, opponent, cx.source_id(), &cost)? {
        return Some(());
    }
    let you = cx.controller();
    let q = Query::YesNo { message: format!("Copy {}?", game.obj(spell).name()) };
    let yes = loop {
        match ask(game, d, you, &q) {
            Answer::Bool(b) => break b,
            Answer::Stop => return None,
            _ => {}
        }
    };
    if yes {
        copy_spell_with(game, d, &original, you, true, false)?;
    }
    Some(())
}

/// `ExploreTheVastlandsTarget`'s filter: a land card, or an instant or sorcery card. (Its `canTarget` override, one
/// of each, isn't the overload `choose` asks: a player choosing picks any two the filter allows, as Java's do.)
fn allowed(game: &Game, c: ObjId) -> bool {
    let o = game.obj(c);
    o.is(CardType::Land) || o.is(CardType::Instant) || o.is(CardType::Sorcery)
}

/// Each player in turn order from you: `choose(cards, ExploreTheVastlandsTarget)` (up to two of the top five, picked
/// one at a time until two are chosen, none is left to choose or the player is done), the reveal, the cards into
/// their hand, the rest on the bottom in a random order
fn explore(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let cards: Vec<ObjId> = game.player(p).library.iter().take(5).copied().collect();
        let mut chosen: Vec<ObjId> = Vec::new();
        if !cards.is_empty() {
            loop {
                let possible: Vec<ObjId> = cards.iter().copied().filter(|c| allowed(game, *c)).collect();
                match ask(game, d, p, &Query::Cards { objs: possible.clone(), required: false }) {
                    Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                    Answer::Obj(o) if possible.contains(&o) => {
                        chosen.push(o);
                        // `isChoiceCompleted`: two chosen, or nothing else to choose
                        if chosen.len() >= 2 || possible.iter().all(|c| chosen.contains(c)) {
                            break;
                        }
                    }
                    Answer::Stop => return None,
                    Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                    _ => break,
                }
                if d.stopped() {
                    return None;
                }
            }
        }
        if !chosen.is_empty() {
            let zcc = game.obj(cx.source_id()).zcc;
            let shown = chosen.iter().map(|c| (*c, Zone::Library)).collect();
            game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
        }
        for c in &chosen {
            game.move_to(*c, Zone::Hand);
        }
        let rest: Vec<ObjId> = cards.into_iter().filter(|c| !chosen.contains(c)).collect();
        if !put_on_library(game, d, p, &rest, false, false) {
            return None;
        }
    }
    Some(())
}
