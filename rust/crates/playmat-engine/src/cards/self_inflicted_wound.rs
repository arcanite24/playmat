//! Self-Inflicted Wound: "Target opponent sacrifices a green or white creature. If that player does, they lose 2
//! life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SelfInflictedWound.SelfInflictedWoundEffect", apply)];

/// the opponent chooses (`TargetSacrifice(green or white creature)`); the life loss when it left the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            let colors = game.colors(*o);
            game.obj(*o).controller == p
                && game.has_type(*o, CardType::Creature)
                && (colors.contains(&Color::G) || colors.contains(&Color::W))
        })
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, p, &possible, &possible, 1, 1, "sacrifice")?;
    let Some(o) = chosen.first().copied() else { return Some(()) };
    sacrifice(game, o);
    if game.obj(o).zone != Zone::Battlefield {
        lose_life(game, p, 2);
    }
    Some(())
}
