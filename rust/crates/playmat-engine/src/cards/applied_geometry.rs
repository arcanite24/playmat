//! Applied Geometry: "Create a token that's a copy of target non-Aura permanent you control, except it's a 0/0
//! Fractal creature in addition to its other types. Put six +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AppliedGeometry.AppliedGeometryEffect", apply)];

/// `CreateTokenCopyTargetEffect(null, CREATURE, false, 1, false, false, null, 0, 0, false)` with a Fractal subtype
/// (as `applyAdditionsToToken` orders them), then six +1/+1 counters on each token
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let of = match cx.item.targets.first().and_then(|t| t.first()) {
        Some(Target::Obj(o)) => *o,
        _ => return Some(()),
    };
    let made = token_copy_modified(game, d, of, you, 1, &|def: &mut CardDef| {
        if !def.types.contains(&CardType::Creature) {
            def.types.push(CardType::Creature);
        }
        def.remove_pt_cda();
        def.power = 0;
        def.toughness = 0;
        if !def.subtypes.iter().any(|s| s == "Fractal") {
            def.subtypes.push("Fractal".into());
        }
    });
    if d.stopped() {
        return None;
    }
    for t in made {
        if game.obj(t).zone == Zone::Battlefield {
            put_counters(game, t, "+1/+1", 6, you);
        }
    }
    Some(())
}
