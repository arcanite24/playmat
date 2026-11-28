//! Shaman of Forgotten Ways: "Formidable — {9}{G}{G}, {T}: Each player's life total becomes the number of creatures
//! they control. Activate only if creatures you control have total power 8 or greater."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShamanOfForgottenWays.ShamanOfForgottenWaysEffect", apply)];

/// Each player in turn (`getPlayersInRange`): `setLife`, which gains or loses the difference (rule 118.5)
fn apply(game: &mut Game, _: &mut dyn Decider, _: &EffectCx) -> Option<()> {
    for p in 0..game.seats() {
        if game.player(p).lost {
            continue;
        }
        let creatures = game
            .battlefield
            .iter()
            .filter(|o| game.obj(**o).controller == p && game.has_type(**o, CardType::Creature))
            .count() as i32;
        let life = game.player(p).life;
        if creatures > life {
            gain_life(game, p, creatures - life);
        } else if creatures < life {
            lose_life(game, p, life - creatures);
        }
    }
    Some(())
}
