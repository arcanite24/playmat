//! Outlaw Stitcher: "When Outlaw Stitcher enters, create a 2/2 blue and black Zombie Rogue creature token, then put
//! two +1/+1 counters on that token for each spell you've cast this turn other than the first. Plot {4}{U}"

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("OutlawStitcher.OutlawStitcherEffect", apply).tokens(&["ZombieRogueToken"])];

/// The Zombie Rogue, then two +1/+1 counters on it for each spell you've cast this turn past the first (counted
/// after it's made)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    if d.stopped() {
        return None;
    }
    let cast = game.watch.spells.iter().filter(|s| s.caster == you).count() as i32;
    let x = (cast - 1).max(0);
    if x <= 0 {
        return Some(());
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        add_counters_by(game, t, "+1/+1", 2 * x, Some(you));
    }
    Some(())
}
