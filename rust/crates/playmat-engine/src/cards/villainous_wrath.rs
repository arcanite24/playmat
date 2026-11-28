//! Villainous Wrath: "Target opponent loses life equal to the number of creatures they control. Then destroy all
//! creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VillainousWrath.VillainousWrathEffect", apply)];

/// The life loss; the destroying is the card's next effect
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let n = game
        .battlefield
        .iter()
        .filter(|&&id| game.obj(id).controller == opponent && game.has_type(id, CardType::Creature))
        .count() as i32;
    if n > 0 {
        lose_life_by(game, opponent, n, Some(cx.source_id()), false);
    }
    Some(())
}
