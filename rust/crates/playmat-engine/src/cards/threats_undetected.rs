//! Threats Undetected: "Search your library for up to four creature cards with different powers and reveal them. An
//! opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ThreatsUndetected.ThreatsUndetectedEffect", apply),
];

/// The opponent (the only one, or `TargetOpponent` chosen), the search (`ThreatsUndetectedTarget`, see `search`);
/// none found, a shuffle. Otherwise they're revealed, the opponent chooses `min(found, 2)` of them (`chooseTarget`), the chosen are put into the library (each on top, in the order
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
    let found = search(game, d, you)?;
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

/// `searchLibrary` with `ThreatsUndetectedTarget` (`TargetCardInLibrary(0, 4, creature cards)`): what can be picked
/// and whether the choice is complete read the filter alone (`possibleTargets` doesn't call the overridden
/// `canTarget`), but a pick whose power one already chosen has isn't added. `TargetCardInLibrary.choose` rounds:
/// another `chooseTarget` after one that added a card and didn't complete the choice
fn search(game: &mut Game, d: &mut dyn Decider, you: Seat) -> Option<Vec<ObjId>> {
    let mut shown = game.player(you).library.clone();
    shown.sort_by(|a, b| game.obj(*a).name().cmp(game.obj(*b).name()));
    game.searching = Some((shown, Vec::new()));
    let possible: Vec<ObjId> = game
        .player(you)
        .library
        .iter()
        .copied()
        .filter(|c| game.obj(*c).def.types.contains(&CardType::Creature))
        .collect();
    let max = 4;
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let before = chosen.len();
        let mut complete = false;
        loop {
            let left: Vec<ObjId> = possible.iter().copied().filter(|c| !chosen.contains(c)).collect();
            match ask(game, d, you, &Query::Cards { objs: left.clone(), required: false }) {
                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                Answer::Obj(o) if left.contains(&o) => {
                    let power = game.obj(o).def.power;
                    if chosen.iter().all(|c| game.obj(*c).def.power != power) {
                        chosen.push(o);
                        if let Some((_, so_far)) = &mut game.searching {
                            so_far.clone_from(&chosen);
                        }
                        if chosen.len() >= max || chosen.len() == possible.len() {
                            complete = true;
                            break;
                        }
                    }
                }
                Answer::Stop => {
                    game.searching = None;
                    return None;
                }
                Answer::Bool(_) => break,
                _ => {}
            }
            if d.stopped() {
                game.searching = None;
                return None;
            }
        }
        if complete || chosen.is_empty() || chosen.len() == before {
            game.searching = None;
            // `LibrarySearchedEvent`
            let sources = game.trigger_sources();
            let fired =
                game.triggers(sources, |_, _, p, t| matches!(t, Trigger::OpponentSearchesLibrary) && p != you);
            game.pending.extend(fired);
            return Some(chosen);
        }
    }
}
