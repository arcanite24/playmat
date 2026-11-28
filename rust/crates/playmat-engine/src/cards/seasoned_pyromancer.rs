//! Seasoned Pyromancer: "When Seasoned Pyromancer enters the battlefield, discard two cards, then draw two cards.
//! For each nonland card discarded this way, create a 1/1 red Elemental creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SeasonedPyromancer.SeasonedPyromancerEffect", apply)
    .tokens(&["RedElementalToken"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let discarded = discard(game, d, you, 2);
    if d.stopped() {
        return None;
    }
    let nonlands = discarded.iter().filter(|c| !game.obj(**c).def.is(CardType::Land)).count() as i32;
    draw(game, you, 2);
    if nonlands > 0 {
        if let Some(token) = cx.tokens.first() {
            cx.create_tokens(game, d, token, nonlands);
        }
    }
    (!d.stopped()).then_some(())
}
