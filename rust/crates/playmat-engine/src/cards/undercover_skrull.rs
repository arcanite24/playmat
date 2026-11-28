//! Undercover Skrull: "As long as there are two or more creature cards in your graveyard, this creature gets +2/+2
//! and is all creature types." The second half is `IsAllCreatureTypesSourceEffect` (layer 4) under the
//! `ConditionalContinuousEffect`, while the source is on the battlefield (its static ability's zone). Named for this
//! card only: other cards put that effect in `Zone.ALL`.

use super::gain_all_creature_types_attached_effect::CREATURE_TYPES;
use super::*;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "UndercoverSkrull.IsAllCreatureTypesSourceEffect",
    affects,
    Layer::Type(all_creature_types),
)];

fn affects(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

fn all_creature_types(_: &Game, _: &StaticCx, _: ObjId) -> Vec<String> {
    CREATURE_TYPES.iter().map(|s| s.to_string()).collect()
}
