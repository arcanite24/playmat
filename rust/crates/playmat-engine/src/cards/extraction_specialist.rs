//! Extraction Specialist: "When this creature enters, return target creature card with mana value 2 or less from
//! your graveyard to the battlefield. That creature can't attack or block for as long as you control this
//! creature."
//! The card goes onto the battlefield under your control; if this is still on the battlefield under its ability's
//! controller, `CantAttackBlockTargetEffect(Duration.WhileControlled)` points at the permanent it became: a lasting
//! restriction that stops applying once this leaves or changes control.

use super::*;

const CANT_ATTACK_OR_BLOCK: &str = "ExtractionSpecialist.CantAttackBlockWhileControlled";

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ExtractionSpecialist.ExtractionSpecialistEffect", apply),
    Handler::restriction(
        CANT_ATTACK_OR_BLOCK,
        the_creature,
        &[Restrict::CantAttack(never_attack), Restrict::CantBlock(never_block)],
    )
    .on_start(point)
    .lasting(),
];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let you = cx.controller();
    put_onto_battlefield(game, d, c, you);
    if d.stopped() {
        return None;
    }
    let src = cx.source_id();
    if game.obj(c).zone != Zone::Battlefield
        || game.obj(src).zone != Zone::Battlefield
        || game.obj(src).controller != you
    {
        return Some(());
    }
    let handler = serde_json::from_value(serde_json::json!(CANT_ATTACK_OR_BLOCK)).ok()?;
    cx.run(
        game,
        d,
        &[EffectDef::CardStatic {
            handler,
            args: Vec::new(),
            effects: Vec::new(),
            filters: Vec::new(),
            abilities: Vec::new(),
        }],
    );
    Some(())
}

/// the permanent the target became, then this creature, each as it is now
fn point(game: &mut Game, _: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let mut out = Vec::new();
    if let Some((Target::Obj(c), _)) = cx.targets.first() {
        out.push((Target::Obj(*c), game.obj(*c).zcc));
    }
    out.push((Target::Obj(cx.source), game.obj(cx.source).zcc));
    Some(out)
}

/// that creature, while you control this one (the same permanent)
fn the_creature(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.target_obj(game, 0) == Some(id)
        && cx.target_obj(game, 1).is_some_and(|s| {
            game.obj(s).zone == Zone::Battlefield && game.obj(s).controller == cx.controller
        })
}

fn never_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn never_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
