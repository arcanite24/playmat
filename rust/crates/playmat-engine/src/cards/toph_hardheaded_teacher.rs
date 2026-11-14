//! Toph, Hardheaded Teacher: "Whenever you cast a spell, earthbend 1. If that spell is a Lesson, put an additional
//! +1/+1 counter on that land."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TophHardheadedTeacher.TophHardheadedTeacherEffect", apply)
    .effects(&["new AddCountersTargetEffect(CounterType.P1P1.createInstance())"])];

/// the spell cast (`getValue("spellCast")`) is a Lesson: one more +1/+1 counter on the target land
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    if cx.obj(0).is_some() && game.has_subtype(spell, "Lesson") {
        cx.run(game, d, &cx.effects[..1]);
    }
    Some(())
}
