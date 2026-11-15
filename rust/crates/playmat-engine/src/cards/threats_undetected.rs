//! Threats Undetected: "Search your library for up to four creature cards with different powers and reveal them. An
//! opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ThreatsUndetected.ThreatsUndetectedEffect", apply),
    // `ThreatsUndetectedTarget.canTarget`: a power none of the cards chosen so far has (the search going on)
    Handler::predicate("ThreatsUndetected.ThreatsUndetectedTarget", different_power),
];

fn different_power(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let power = game.obj(id).def.power;
    game.searching
        .as_ref()
        .is_none_or(|(_, chosen)| chosen.iter().all(|c| *c == id || game.obj(*c).def.power != power))
}

/// The opponent (the only one, or `TargetOpponent` chosen), the search (`TargetCardInLibrary(0, 4, creature cards)`
/// with different powers); none found, a shuffle. Otherwise they're revealed, the opponent chooses
/// `min(found, 2)` of them (`chooseTarget`), the chosen are put into the library (each on top, in the order
/// chosen) and it's shuffled (`shuffleCardsToLibrary`; a shuffle alone with none chosen), and the rest go to hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let opponents: Vec<Seat> =
        (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    let opp = if opponents.len() == 1 {
        opponents[0]
    } else {
        let chosen = choose_target(
            game,
            d,
            you,
            &crate::card::TargetDef::TargetOpponent,
            Some(cx.source_id()),
            &rule,
            false,
        );
        if d.stopped() {
            return None;
        }
        match chosen.and_then(|c| c.first().copied()) {
            Some(Target::Player(p)) => p,
            _ => return Some(()),
        }
    };
    let filter: crate::card::Filter = serde_json::from_value(serde_json::json!([
        {"p": "type", "t": "CREATURE"},
        {"p": "card", "handler": "ThreatsUndetected.ThreatsUndetectedTarget"}
    ]))
    .ok()?;
    let found = search_library(game, d, you, &filter, 0, 4, cx.source_id())?;
    if found.is_empty() {
        cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
        return Some(());
    }
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc: cx.source.1,
        paren: false,
        cards: found.iter().map(|c| (*c, game.obj(*c).zone)).collect(),
    });
    let n = found.len().min(2);
    let chosen = choose_cards(game, d, opp, &found, &found, n, n, &rule)?;
    if chosen.is_empty() {
        cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    } else {
        for c in &chosen {
            to_library(game, *c, true);
        }
        cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    }
    let rest: Vec<ObjId> = found.iter().copied().filter(|c| !chosen.contains(c)).collect();
    for c in rest {
        if game.obj(c).zone == Zone::Library {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}
