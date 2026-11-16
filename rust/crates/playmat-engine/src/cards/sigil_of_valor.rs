//! Sigil of Valor: "Whenever equipped creature attacks alone, it gets +1/+1 until end of turn for each other
//! creature you control." `SigilOfValorCount`: the creatures you control other than the effect's target (the
//! attacker the trigger points at).

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::value("SigilOfValor.SigilOfValorCount", others)];

fn others(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let Some(Target::Obj(attacker)) = game.resolving_item.as_ref().and_then(|i| i.targets.first()?.first().copied())
    else {
        return 0;
    };
    game.battlefield
        .iter()
        .filter(|c| **c != attacker && game.obj(**c).controller == you && game.has_type(**c, CardType::Creature))
        .count() as i32
}
