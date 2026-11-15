//! Armor Wars: "I — You may draw a card for each artifact you control. If you do, each opponent draws a card.
//! II — Artifact spells you cast this turn cost {1} less to cast."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ArmorWars.ArmorWarsEffect", draw_for_artifacts),
    Handler::cost("ArmorWars.SpellsCostReductionControllerEffect", CostKind::Reduce, artifact_spell, one_less)
        .args(&["one_of:filterArtifactCard", "one_of:1"]),
];

/// chapter II, until the turn's cleanup: artifact spells you cast
fn artifact_spell(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller && of.face.is(CardType::Artifact)
}

/// `CardUtil.reduceCost(ability, 1)`: one generic
fn one_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}

/// `ArtifactYouControlCount`: none, nothing asked; else "Draw a card for each artifact you control? (each opponent
/// also draws a card)", then you draw that many and each opponent one
fn draw_for_artifacts(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|a| game.obj(**a).controller == you && game.has_type(**a, CardType::Artifact))
        .count() as i32;
    if n <= 0
        || !choose_use(game, d, you, "Draw a card for each artifact you control? (each opponent also draws a card)")?
    {
        return Some(());
    }
    draw(game, you, n as usize);
    let opponent = game.opponent(you);
    draw(game, opponent, 1);
    Some(())
}
