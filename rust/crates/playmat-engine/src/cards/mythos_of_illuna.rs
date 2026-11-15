//! Mythos of Illuna: "Create a token that's a copy of target permanent. If {R}{G} was spent to cast this spell,
//! instead create a token that's a copy of that permanent, except the token has "When this token enters, if it's a
//! creature, it fights up to one target creature you don't control.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MythosOfIlluna.MythosOfIllunaEffect", apply)];

/// The target still on the battlefield is copied (`CreateTokenCopyTargetEffect`); with {R} and {G} spent to cast
/// this spell (`ManaWasSpentCondition.RED` and `GREEN`), the token has the fight trigger too (an intervening "if
/// it's a creature": `SourceMatchesFilterCondition`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let spent = &game.obj(cx.source_id()).spent_colors;
    let rg = spent[3] > 0 && spent[4] > 0;
    let mut e = serde_json::json!({"class": "CreateTokenCopyTargetEffect"});
    if rg {
        e["abilities"] = serde_json::json!([{
            "class": "TriggeredAbility",
            "on": {"e": "enters"},
            "condition": {"c": "source_matches", "filter": [{"p": "type", "t": "CREATURE"}]},
            "effects": [{"class": "FightTargetSourceEffect"}],
            "rule": "When this token enters, if it's a creature, it fights up to one target creature you don't control.",
            "targets": [{
                "class": "TargetPermanent",
                "filter": [{"p": "type", "t": "CREATURE"}, {"p": "controller", "who": "opponent"}],
                "min": 0,
                "max": 1
            }]
        }]);
    }
    let effect: EffectDef = serde_json::from_value(e).ok()?;
    cx.run(game, d, &[effect]);
    Some(())
}
