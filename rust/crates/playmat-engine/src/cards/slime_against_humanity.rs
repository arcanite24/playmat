//! Slime Against Humanity: "Create a 0/0 green Ooze creature token with trample. Put X +1/+1 counters on it, where
//! X is two plus the total number of cards you own in exile and in your graveyard that are Oozes or are named Slime
//! Against Humanity. A deck can have any number of cards named Slime Against Humanity."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SlimeAgainstHumanity.SlimeAgainstHumanityEffect", apply)
    .effects(&["new CreateTokenEffect(new OozeTrampleToken())"])];

/// the token, then X counted (the token on the battlefield by then) and put on each token made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    let slime = |game: &Game, c: ObjId| game.has_subtype(c, "Ooze") || game.obj(c).name() == "Slime Against Humanity";
    let grave = game.player(you).graveyard.iter().filter(|c| slime(game, **c)).count() as i32;
    let exiled = game.exile.iter().filter(|c| game.obj(**c).owner == you && slime(game, **c)).count() as i32;
    let x = 2 + grave + exiled;
    for t in made {
        add_counters_by(game, t, "+1/+1", x, Some(you));
    }
    Some(())
}
