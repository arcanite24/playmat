//! Rakdos, the Showstopper: "When Rakdos enters, flip a coin for each creature that isn't a Demon, Devil, or Imp.
//! Destroy each creature whose coin comes up tails."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RakdosTheShowstopper.RakdosTheShowstopperEffect", apply)];

/// Each such creature in battlefield order: `flipCoin(source, game, false)` (not winnable: no call, the flip's
/// result), and tails destroys it at once
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game.battlefield.iter().copied().collect();
    for o in creatures {
        if game.obj(o).zone != Zone::Battlefield
            || !game.has_type(o, CardType::Creature)
            || ["Demon", "Devil", "Imp"].iter().any(|s| game.has_subtype(o, s))
        {
            continue;
        }
        let heads = game.rng.next_boolean();
        if log::kept(game) {
            let from = log::source(game).map(|s| format!(" (source: {})", log::object(game, s))).unwrap_or_default();
            let line = format!("{} flipped {}{}", log::player(you), if heads { "Heads" } else { "Tails" }, from);
            log::inform(game, line);
        }
        if !heads {
            destroy(game, o);
        }
    }
    Some(())
}
