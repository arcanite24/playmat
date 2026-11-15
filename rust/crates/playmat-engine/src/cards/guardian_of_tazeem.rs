//! Guardian of Tazeem: "Flying. Landfall — Whenever a land you control enters, tap target creature an opponent
//! controls. If that land is an Island, that creature doesn't untap during its controller's next untap step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "GuardianOfTazeem.GuardianOfTazeemTriggeredAbility",
        "new LandfallAbility(new TapTargetEffect())",
        check,
    ),
    Handler::effect("GuardianOfTazeem.GuardianOfTazeemEffect", apply),
];

/// `ENTERS_THE_BATTLEFIELD` of a land you control (the base trigger), its pointer at the land (`entering`)
fn check(_: &Game, _: &StackItem) -> bool {
    true
}

/// The land that entered (as it is, or as it last was) an Island and the target still a permanent: it doesn't untap
/// during its controller's next untap step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(land) = cx.item.entering else { return Some(()) };
    let Some(creature) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let l = game.obj(land);
    let island = match l.zone {
        Zone::Battlefield => game.has_subtype(land, "Island"),
        _ => l.lki_def.as_ref().unwrap_or(&l.def).has_subtype("Island"),
    };
    if island {
        cx.run_on(game, d, &[EffectDef::DontUntapInControllersNextUntapStepTargetEffect], vec![Target::Obj(creature)]);
    }
    (!d.stopped()).then_some(())
}
