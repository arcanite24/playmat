//! Matzalantli, the Great Door: "{4}, {T}: Transform Matzalantli. Activate only if there are four or more permanent
//! types among cards in your graveyard."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("MatzalantliTheGreatDoor.MatzalantliTheGreatDoorCondition", four_permanent_types),
    Handler::value("MatzalantliTheGreatDoor.MatzalantliTheGreatDoorValue", permanent_types),
];

/// `MatzalantliTheGreatDoorValue`: the permanent types among cards in your graveyard
fn permanent_types(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut types: Vec<CardType> = Vec::new();
    for c in game.player(you).graveyard.iter() {
        for t in &game.obj(*c).def.types {
            let permanent = matches!(
                t,
                CardType::Artifact
                    | CardType::Battle
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
            );
            if permanent && !types.contains(t) {
                types.push(*t);
            }
        }
    }
    types.len() as i32
}

fn four_permanent_types(game: &Game, you: Seat, source: ObjId) -> bool {
    permanent_types(game, you, source, 0) >= 4
}
