//! Tezzeret, Cruel Captain's 0: "Untap target artifact or creature. If it's an artifact creature, put a +1/+1 counter
//! on it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TezzeretCruelCaptain.TezzeretCruelCaptainEffect", apply),
    Handler::effect("TezzeretCruelCaptainEmblem.TezzeretCruelCaptainEmblemEffect", emblem),
];

/// The emblem's "If it's not a creature, it becomes a 0/0 Robot artifact creature" (card types and the subtype
/// added, base power and toughness 0/0, until it leaves: `Duration.Custom`)
fn emblem(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.has_type(o, CardType::Creature) {
        return Some(());
    }
    let token: TokenDef = serde_json::from_value(serde_json::json!({
        "name": "", "types": ["ARTIFACT", "CREATURE"], "subtypes": ["Robot"], "power": 0, "toughness": 0
    }))
    .ok()?;
    let e = EffectDef::BecomesCreatureTargetEffect {
        token,
        duration: crate::card::Duration::WhileOnBattlefield,
        lose_abilities: false,
        still_land: true,
        named: None,
    };
    cx.run(game, d, &[e]);
    Some(())
}

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield && game.has_type(o, CardType::Artifact) && game.has_type(o, CardType::Creature)
    {
        add_counters_by(game, o, "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}
