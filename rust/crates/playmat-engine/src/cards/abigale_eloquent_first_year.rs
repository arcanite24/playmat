//! Abigale, Eloquent First-Year: "When Abigale enters, up to one other target creature loses all abilities. Put a
//! flying counter, a first strike counter, and a lifelink counter on that creature."
//!
//! `LoseAllAbilitiesTargetEffect(Duration.Custom)`: for as long as it stays on the battlefield. Java applies keyword
//! counters' abilities first in layer 6 (`ApplyStatusEffect`), so the counters put after it give nothing; statics
//! with earlier timestamps (an Aura's "enchanted creature has flying") give nothing either (`lost_all_at`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AbigaleEloquentFirstYear.LoseAllAbilitiesTargetEffect", apply)
    .args(&["one_of:Duration.Custom"])];

fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let at = game.timestamp();
        let ob = game.obj_mut(o);
        ob.lost_all_at = at;
        if let Some(def) = ob.becomes_base.take() {
            ob.def = def;
        }
        let old = ob.def.clone();
        let mut def = (*old).clone();
        def.abilities.clear();
        ob.def = Arc::new(def);
        // (an end-of-turn change undone at cleanup doesn't bring them back)
        if let Some(eot) = ob.until_end_of_turn_def.as_mut() {
            Arc::make_mut(eot).abilities.clear();
        }
        ob.lost_all = 2;
        // its abilities already waiting or on the stack are independent of it (113.7a): they keep theirs
        for t in Arc::make_mut(&mut game.stack).iter_mut().chain(game.pending.iter_mut()) {
            if t.card == o && t.ability.is_some() && t.def.is_none() {
                t.def = Some(old.clone());
            }
        }
    }
    Some(())
}
