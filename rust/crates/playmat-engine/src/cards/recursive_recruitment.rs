//! Recursive Recruitment: "Create two 2/2 colorless Wizard Soldier creature tokens named Cadet. If this spell was
//! cast from a graveyard, put a +1/+1 counter on each of them for every three cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RecursiveRecruitment.RecursiveRecruitmentEffect", apply)
    .effects(&["new CreateTokenEffect(new CadetToken(), 2)"])];

/// The two Cadets (`getLastAddedTokenIds`: the tokens made now), then on each still on the battlefield a +1/+1
/// counter for every three cards in your graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let first = game.objs.len();
    cx.run(game, d, cx.effects);
    let made: Vec<ObjId> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        let n = game.player(you).graveyard.len() as i32 / 3;
        if n > 0 {
            add_counters_by(game, t, "+1/+1", n, Some(you));
        }
    }
    Some(())
}
