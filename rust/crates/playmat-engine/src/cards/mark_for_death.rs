//! Mark for Death: "Target creature an opponent controls blocks this turn if able. Untap that creature. Other
//! creatures that player controls can't block this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("MarkForDeath.MarkForDeathEffect", apply)
        .effects(&["new BlocksIfAbleTargetEffect(Duration.EndOfTurn)"]),
    // made by the effect (`args`: the target's controller's seat, the target): `CantBlockAllEffect` of the creatures
    // that player controls other than the target, until end of turn
    Handler::restriction("MarkForDeath.MarkForDeathCantBlockEffect", others_of_that_player, &[Restrict::CantBlock(any)]),
];

/// the target still a permanent: it blocks this turn if able, it untaps, then the other creatures its controller
/// controls can't block this turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.obj(0) else { return Some(()) };
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let controller = game.obj(target).controller;
    cx.run(game, d, &cx.effects[..1]);
    untap_permanent(game, target);
    let handler = serde_json::from_value(serde_json::json!("MarkForDeath.MarkForDeathCantBlockEffect")).ok()?;
    let e = EffectDef::CardStatic {
        handler,
        args: vec![controller.to_string(), target.0.to_string()],
        effects: Vec::new(),
        filters: Vec::new(),
        abilities: Vec::new(),
    };
    cx.run_on(game, d, &[e], Vec::new());
    Some(())
}

/// a creature that player controls, not the target
fn others_of_that_player(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let (Some(seat), Some(target)) = (cx.args.first(), cx.args.get(1)) else { return false };
    game.obj(id).controller.to_string() == *seat && id.0.to_string() != *target && game.has_type(id, CardType::Creature)
}

fn any(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
