//! Vastlands Scavenger // Bind to Life: "Mill seven cards. Then put a creature card from among them onto the
//! battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VastlandsScavenger.VastlandsScavengerEffect", apply)];

/// `millCards(7)`, then the creature cards among them in the graveyard: one is put onto the battlefield (a single
/// one without asking), chosen when there are two or more
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    let creatures: Vec<ObjId> = top
        .iter()
        .copied()
        .filter(|c| game.obj(*c).zone == Zone::Graveyard && game.has_type(*c, CardType::Creature))
        .collect();
    let card = match creatures.len() {
        0 => return Some(()),
        1 => creatures[0],
        _ => {
            let rule = "put a creature card from among them onto the battlefield";
            match choose_cards(game, d, you, &creatures, &creatures, 1, 1, rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    put_onto_battlefield(game, d, card, you);
    if d.stopped() {
        return None;
    }
    Some(())
}
