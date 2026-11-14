//! Wandering Archaic: "Whenever an opponent casts an instant or sorcery spell, they may pay {2}. If they don't, you
//! may copy that spell. You may choose new targets for the copy."
//! Explore the Vastlands: "Each player looks at the top five cards of their library and may reveal a land card and/or
//! an instant or sorcery card from among them. Each player puts the cards they revealed this way into their hand and
//! the rest on the bottom of their library in a random order. Each player gains 3 life."

use super::*;
use crate::card::CostDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("WanderingArchaic.WanderingArchaicEffect", apply),
    Handler::effect("WanderingArchaic.ExploreTheVastlandsEffect", explore_the_vastlands),
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

/// each player in turn, from you: the top five, `player.choose` of up to two (`ExploreTheVastlandsTarget`: a land
/// and an instant or sorcery, not two of a kind), into the hand, the rest on the bottom in a random order
fn explore_the_vastlands(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        let cards: Vec<ObjId> = game.player(p).library.iter().take(5).copied().collect();
        let chosen = choose_land_and_spell(game, d, p, &cards)?;
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

/// a land, or an instant or sorcery
fn kind(game: &Game, c: ObjId) -> (bool, bool) {
    let o = game.obj(c);
    (o.is(CardType::Land), o.is(CardType::Instant) || o.is(CardType::Sorcery))
}

/// `HumanPlayer.choose(cards, TargetCardInLibrary(0, 2))` with the target's `canTarget`: a card of the filter that
/// isn't of a kind already chosen; done when nothing more can be chosen
fn choose_land_and_spell(game: &mut Game, d: &mut dyn Decider, seat: Seat, cards: &[ObjId]) -> Option<Vec<ObjId>> {
    if cards.is_empty() {
        return Some(Vec::new());
    }
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        if chosen.len() >= 2 {
            return Some(chosen);
        }
        let can = |game: &Game, chosen: &[ObjId], c: ObjId| {
            let (land, spell) = kind(game, c);
            (land || spell)
                && !chosen.contains(&c)
                && chosen.iter().all(|x| {
                    let (l, s) = kind(game, *x);
                    !(land && l || spell && s)
                })
        };
        let left: Vec<ObjId> = cards.iter().copied().filter(|c| can(game, &chosen, *c)).collect();
        match ask(game, d, seat, &Query::Cards { objs: left.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if left.contains(&o) => {
                chosen.push(o);
                // `isChoiceCompleted`: nothing more to choose
                if !cards.iter().any(|c| can(game, &chosen, *c)) {
                    return Some(chosen);
                }
            }
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ => return Some(chosen),
        }
        if d.stopped() {
            return None;
        }
    }
}
