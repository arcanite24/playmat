//! Mutagen Man, Living Ooze: "Activated abilities of artifact tokens you control cost {1} less to activate."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("MutagenManLivingOoze.MutagenManLivingOozeEffect", CostKind::Reduce, applies, reduce)];

/// an activated ability of an artifact token you control
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_some()
        && game.has_type(of.card, CardType::Artifact)
        && game.obj(of.card).token
        && game.obj(of.card).controller == cx.controller
}

/// `CardUtil.reduceCost(ability, 1)`: one generic mana less
fn reduce(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}
