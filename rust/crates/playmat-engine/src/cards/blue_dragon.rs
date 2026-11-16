//! Blue Dragon: "Flying. Lightning Breath — When Blue Dragon enters the battlefield, until your next turn, target
//! creature an opponent controls gets -3/-0, up to one other target creature gets -2/-0, and up to one other target
//! creature gets -1/-0."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlueDragon.BlueDragonEffect", apply).effects(&[
    "new BoostTargetEffect(-3, 0, Duration.UntilYourNextTurn)",
    "new BoostTargetEffect(-2, 0, Duration.UntilYourNextTurn)",
    "new BoostTargetEffect(-1, 0, Duration.UntilYourNextTurn)",
])];

/// Java's one continuous effect over the ability's three targets, in order: -3/-0, -2/-0 and -1/-0 until your next
/// turn, each to its target's creature (still the creature it targeted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for (i, e) in cx.effects.iter().enumerate().take(3) {
        let Some(o) = cx.obj(i) else { continue };
        if game.obj(o).zone != Zone::Battlefield || !game.has_type(o, CardType::Creature) {
            continue;
        }
        cx.run_on(game, d, std::slice::from_ref(e), vec![Target::Obj(o)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
