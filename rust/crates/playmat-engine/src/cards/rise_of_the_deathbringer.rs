//! Rise of the Deathbringer: "Draw cards equal to the greatest power among creatures you control. You lose life
//! equal to the number of cards drawn this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RiseOfTheDeathbringer.RiseOfTheDeathbringerDrawEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let greatest = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == you && game.has_type(**b, CardType::Creature))
        .map(|b| game.power_toughness(*b).0)
        .max()
        .unwrap_or(0);
    if greatest > 0 {
        let before = game.player(you).library.len();
        draw(game, you, greatest as usize);
        let drawn = before.saturating_sub(game.player(you).library.len()) as i32;
        if drawn > 0 {
            lose_life(game, you, drawn);
        }
    }
    Some(())
}
