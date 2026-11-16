//! `GainAllCreatureTypesTargetEffect(Duration.EndOfTurn)` (layer 4): "until end of turn, that creature … gains all
//! creature types" (Glamer Gifter): the resolving effect's target has every creature type until the cleanup.

use super::gain_all_creature_types_attached_effect::CREATURE_TYPES;
use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("GainAllCreatureTypesTargetEffect", affects, Layer::Type(all_creature_types))
        .args(&["one_of:Duration.EndOfTurn"])];

fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.target_obj(game, 0) == Some(id)
}

fn all_creature_types(_: &Game, _: &StaticCx, _: ObjId) -> Vec<String> {
    CREATURE_TYPES.iter().map(|s| s.to_string()).collect()
}
