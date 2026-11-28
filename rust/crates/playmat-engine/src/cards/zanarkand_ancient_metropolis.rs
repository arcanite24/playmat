//! Zanarkand, Ancient Metropolis's adventure, Lasting Fayth: "Create a 1/1 colorless Hero creature token. Put a +1/+1
//! counter on it for each land you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZanarkandAncientMetropolis.ZanarkandAncientMetropolisEffect", apply)
    .effects(&["new CreateTokenEffect(new HeroToken())"])];

/// the token first, then the lands are counted, then the counters on the tokens made (`getLastAddedTokenIds`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let n = game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land)).count()
        as i32;
    if n < 1 {
        return Some(());
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        put_counters(game, t, "+1/+1", n, you);
    }
    Some(())
}
