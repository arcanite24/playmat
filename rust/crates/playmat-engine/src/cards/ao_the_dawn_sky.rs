//! Ao, the Dawn Sky: "When Ao dies, choose one — • Look at the top seven cards of your library. Put any number of
//! nonland permanent cards with total mana value 4 or less from among them onto the battlefield. Put the rest on the
//! bottom of your library in a random order. • Put two +1/+1 counters on each permanent you control that's a creature
//! or Vehicle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AoTheDawnSky.AoTheDawnSkyEffect", apply)];

/// `AoTheDawnSkyTarget` (0 to any, total mana value 4 or less) in one `choose`: the cards that still fit are offered
/// after each pick, until none fits or the player is done
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    let mut chosen: Vec<ObjId> = Vec::new();
    if !cards.is_empty() {
        let fits = |game: &Game, chosen: &[ObjId], c: ObjId| {
            let used: u32 = chosen.iter().map(|x| game.obj(*x).def.mana_value()).sum();
            let o = game.obj(c);
            o.def.is_permanent_card() && !o.def.types.contains(&CardType::Land) && used + o.def.mana_value() <= 4
        };
        loop {
            let left: Vec<ObjId> = cards.iter().copied().filter(|c| !chosen.contains(c) && fits(game, &chosen, *c)).collect();
            if !chosen.is_empty() && left.is_empty() {
                break;
            }
            match ask(game, d, you, &Query::Cards { objs: left.clone(), required: false }) {
                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                Answer::Obj(o) if left.contains(&o) => chosen.push(o),
                Answer::Stop => return None,
                Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                _ => break,
            }
            if d.stopped() {
                return None;
            }
        }
    }
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}
