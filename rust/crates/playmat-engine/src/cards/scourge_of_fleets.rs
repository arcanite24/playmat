//! Scourge of Fleets: "When Scourge of Fleets enters, return each creature your opponents control with toughness X or
//! less to its owner's hand, where X is the number of Islands you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ScourgeOfFleets.ScourgeOfFleetsEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let islands =
        game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Island")).count()
            as i32;
    let back: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller != you
                && game.has_type(*o, CardType::Creature)
                && game.power_toughness(*o).1 <= islands
        })
        .collect();
    for o in back {
        game.move_to(o, Zone::Hand);
    }
    Some(())
}
