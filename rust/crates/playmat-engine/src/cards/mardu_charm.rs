//! Mardu Charm: "• Create two 1/1 white Warrior creature tokens. They gain first strike until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MarduCharm.MarduCharmCreateTokenEffect", apply)
    .effects(&["new CreateTokenEffect(new WarriorToken(), 2)"])];

/// the tokens made, each granted first strike until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run(game, d, cx.effects);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for o in made {
        grant(game, o, &AbilityDef::FirstStrikeAbility);
    }
    Some(())
}
