//! Grim Contest: "Choose target creature you control and target creature an opponent controls. Each of those
//! creatures deals damage equal to its toughness to the other."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GrimContest.GrimContestEffect", apply)];

/// both must still be there; the second's toughness first, then the first's (as it is after the first damage)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let on = |g: &Game, o: Option<ObjId>| {
        o.filter(|o| g.obj(*o).zone == Zone::Battlefield && g.has_type(*o, CardType::Creature))
    };
    let (Some(a), Some(b)) = (on(game, cx.obj(0)), on(game, cx.obj(1))) else { return Some(()) };
    let tb = game.power_toughness(b).1;
    deal_damage(game, b, Target::Obj(a), tb, false);
    let ta = game.power_toughness(a).1;
    deal_damage(game, a, Target::Obj(b), ta, false);
    Some(())
}
