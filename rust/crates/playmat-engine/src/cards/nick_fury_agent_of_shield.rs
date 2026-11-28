//! Nick Fury, Agent of S.H.I.E.L.D.'s power-up: "…, then look at the top seven cards of your library. You may put a
//! Hero, Equipment, or Vehicle card from among them onto the battlefield. If it's a double-faced card, you may
//! transform it. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NickFuryAgentOfSHIELD.NickFuryAgentOfSHIELDEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    let chosen = if cards.is_empty() {
        Vec::new()
    } else {
        let possible: Vec<ObjId> = cards
            .iter()
            .copied()
            .filter(|c| ["Hero", "Equipment", "Vehicle"].iter().any(|s| game.has_subtype(*c, s)))
            .collect();
        let rule = item_rule(game, cx.item);
        choose_cards(game, d, you, &cards, &possible, 0, 1, &rule)?
    };
    if let Some(&c) = chosen.first() {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
        if game.obj(c).zone == Zone::Battlefield && game.obj(c).controller == you && game.obj(c).def.transform_back.is_some() {
            let name = game.obj(c).name().to_string();
            if choose_use(game, d, you, &format!("Transform {name}?"))? {
                transform(game, c);
            }
        }
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}
