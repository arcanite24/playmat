//! Resolute Archangel: "When this creature enters, if your life total is less than your starting life total, it
//! becomes equal to your starting life total."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ResoluteArchangel.ResoluteArchangelEffect", apply),
    Handler::condition("ResoluteArchangel.ControllerLifeLowerThanStrtingLife", lower),
];

/// `controller.setLife(game.getStartingLife())`: the difference gained or lost (rule 118.5)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let life = game.player(you).life;
    let start = game.starting_life;
    if start > life {
        gain_life(game, you, start - life);
    } else if start < life {
        lose_life(game, you, life - start);
    }
    Some(())
}

fn lower(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).life < game.starting_life
}
