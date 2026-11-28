//! Engulf the Shore: "Return to their owners' hands all creatures with toughness less than or equal to the number of
//! Islands you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EngulfTheShore.EngulfTheShoreEffect", apply)];

/// the Islands counted first, then every such creature to its owner's hand together
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let islands =
        game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Island")).count()
            as i32;
    let bounced: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && game.power_toughness(*o).1 <= islands)
        .collect();
    for o in bounced {
        game.move_to(o, Zone::Hand);
    }
    Some(())
}
