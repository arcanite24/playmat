//! Zul Ashur, Lich Lord: "{T}: You may cast target Zombie creature card from your graveyard this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZulAshurLichLord.PlayFromNotOwnHandZoneTargetEffect", apply)];

/// `PlayFromNotOwnHandZoneTargetEffect(GRAVEYARD, YOU, EndOfTurn, false, onlyCast)`: the engine's "you may cast the
/// target from your graveyard this turn" (Emry's)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &[EffectDef::MayCastTargetFromGraveyardThisTurnEffect]);
    Some(())
}
