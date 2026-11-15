//! Diluvian Primordial: "Flying. When Diluvian Primordial enters, for each opponent, you may cast up to one target
//! instant or sorcery card from that player's graveyard without paying its mana cost. If a spell cast this way would
//! be put into a graveyard, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiluvianPrimordial.DiluvianPrimordialEffect", apply)];

/// Each target card (in a duel, the one from the opponent's graveyard): `MayCastTargetCardEffect(WITHOUT_PAYING_MANA_COST,
/// true)` on it, "Cast X?" and then the cast, exiled instead of going to a graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for i in 0..cx.legal.len() {
        let Some(card) = cx.obj(i) else { continue };
        let e = EffectDef::MayCastTargetCardEffect { any_mana: false, free: true, then_exile: true };
        cx.run_on(game, d, &[e], vec![Target::Obj(card)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
