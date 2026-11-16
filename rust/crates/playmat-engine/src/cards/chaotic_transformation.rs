//! Chaotic Transformation: "Exile up to one target artifact, up to one target creature, up to one target
//! enchantment, up to one target planeswalker, and/or up to one target land. For each permanent exiled this way, its
//! controller reveals cards from the top of their library until they reveal a card that shares a card type with it,
//! puts that card onto the battlefield, then shuffles."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChaoticTransformation.ChaoticTransformationEffect", apply)];

const TYPES: [CardType; 9] = [
    CardType::Artifact,
    CardType::Battle,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Instant,
    CardType::Kindred,
    CardType::Land,
    CardType::Planeswalker,
    CardType::Sorcery,
];

/// Each target group in order: its permanent exiled (as it is now, its types and controller kept); if it went, that
/// player reveals down to a card sharing one of those types, puts it onto the battlefield, then shuffles
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for i in 0..cx.legal.len() {
        let Some(o) = cx.obj(i) else { continue };
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let p = game.obj(o).controller;
        let types: Vec<CardType> = TYPES.iter().copied().filter(|t| game.has_type(o, *t)).collect();
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone != Zone::Exile {
            continue;
        }
        let mut revealed = Vec::new();
        let mut found = None;
        for c in game.player(p).library.clone() {
            revealed.push(c);
            if types.iter().any(|t| game.obj(c).is(*t)) {
                found = Some(c);
                break;
            }
        }
        if !revealed.is_empty() {
            let zcc = game.obj(cx.source_id()).zcc;
            let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
            game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
        }
        if let Some(c) = found {
            put_onto_battlefield(game, d, c, p);
            if d.stopped() {
                return None;
            }
        }
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
