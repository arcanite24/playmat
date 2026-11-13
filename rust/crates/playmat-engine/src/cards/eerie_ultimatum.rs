//! Eerie Ultimatum: "Return any number of permanent cards with different names from your graveyard to the
//! battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EerieUltimatum.EerieUltimatumEffect", apply)];

/// `choose(graveyard, EerieUltimatumTarget)`: up to any number of permanent cards, each pick leaving out the cards
/// that share a name with one chosen; done once nothing more can be chosen (`isChoiceCompleted`) or the player says
/// so. An empty graveyard isn't asked about. They enter together.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let graveyard = game.player(you).graveyard.clone();
    if graveyard.is_empty() {
        return Some(());
    }
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let names: Vec<String> = chosen.iter().map(|c| game.obj(*c).name().to_string()).collect();
        let possible: Vec<ObjId> = graveyard
            .iter()
            .copied()
            .filter(|c| game.obj(*c).def.is_permanent_card() && !names.iter().any(|n| n == game.obj(*c).name()))
            .collect();
        match ask(game, d, you, &Query::Cards { objs: possible.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if possible.contains(&o) => {
                chosen.push(o);
                let names: Vec<String> = chosen.iter().map(|c| game.obj(*c).name().to_string()).collect();
                let more = graveyard.iter().any(|c| {
                    !chosen.contains(c)
                        && game.obj(*c).def.is_permanent_card()
                        && !names.iter().any(|n| n == game.obj(*c).name())
                });
                if !more {
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
    if !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
