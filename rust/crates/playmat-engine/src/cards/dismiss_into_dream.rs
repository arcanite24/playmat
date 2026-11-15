//! Dismiss into Dream: "Each creature your opponents control is an Illusion in addition to its other types and has
//! \"When this creature becomes the target of a spell or ability, sacrifice it.\""

use super::*;
use std::sync::OnceLock;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("DismissIntoDream.DismissIntoDreamEffect", affects, Layer::Type(illusion))
        .args(&["filter"])
        .and_layers(&[Layer::Ability(sacrifice_when_targeted)])];

/// the filter: each creature your opponents control
fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.matches(game, 0, id)
}

fn illusion(_: &Game, _: &StaticCx, _: ObjId) -> Vec<String> {
    vec!["Illusion".into()]
}

fn sacrifice_when_targeted(_: &Game, _: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    sacrifice_when_targeted_def().into_iter().collect()
}

/// `new BecomesTargetSourceTriggeredAbility(new SacrificeSourceEffect())`, with its rule text
fn sacrifice_when_targeted_def() -> Option<AbilityDef> {
    static ABILITY: OnceLock<Option<AbilityDef>> = OnceLock::new();
    ABILITY
        .get_or_init(|| {
            serde_json::from_value(serde_json::json!({
                "class": "TriggeredAbility",
                "effects": [{"class": "SacrificeSourceEffect"}],
                "on": {"abilities": true, "e": "becomes_target_of_spell", "filter": []},
                "optional": false,
                "set_target_pointer": false,
                "rule": "When {this} becomes the target of a spell or ability, sacrifice {this}."
            }))
            .ok()
        })
        .clone()
}

#[cfg(test)]
mod tests {
    #[test]
    fn granted_trigger_parses() {
        assert!(super::sacrifice_when_targeted_def().is_some());
    }
}
