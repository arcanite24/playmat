//! Vivien, Monsters' Advocate: "You may look at the top card of your library any time. You may cast creature spells
//! from the top of your library. +1: Create a 3/3 green Beast creature token. Put your choice of a vigilance counter,
//! a reach counter, or a trample counter on it. −2: When you cast your next creature spell this turn, search your
//! library for a creature card with lesser mana value, put it onto the battlefield, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("VivienMonstersAdvocate.VivienMonstersAdvocateTokenEffect", token).tokens(&["BeastToken"]),
    Handler::trigger(
        "VivienMonstersAdvocate.VivienMonstersAdvocateTriggeredAbility",
        "new SpellCastControllerTriggeredAbility(new VivienMonstersAdvocateSearchEffect(), StaticFilters.FILTER_SPELL_A_CREATURE, false)",
        creature_spell,
    ),
    // (the effect its `checkTrigger` sets: `SearchLibraryPutInPlayEffect` of a creature card with lesser mana value)
    Handler::effect("VivienMonstersAdvocate.VivienMonstersAdvocateSearchEffect", search),
];

/// The Beast token, then for each token made: "Choose vigilance, reach, or trample counter" and one of it
fn token(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, cx.tokens.first()?, 1);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    let choices: Vec<String> = ["Vigilance", "Reach", "Trample"].iter().map(|c| c.to_string()).collect();
    let q = Query::Choice { message: "Choose vigilance, reach, or trample counter".into(), choices: choices.clone() };
    for o in made {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
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
        put_counters(game, o, &chosen.to_lowercase(), 1, you);
    }
    Some(())
}

/// a creature spell you cast (the base trigger is all of it)
fn creature_spell(_: &Game, _: &StackItem) -> bool {
    true
}

/// Search for a creature card with mana value less than the spell's, onto the battlefield, then shuffle
fn search(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mv = cx.item.cast_spell.map_or(0, |s| game.obj(s).def.mana_value() as i32);
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "SearchLibraryEffect",
        "filter": [{"p": "type", "t": "CREATURE"}, {"p": "mv", "cmp": "FEWER_THAN", "n": mv}],
        "min": 1, "max": 1, "to": "BATTLEFIELD",
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}
