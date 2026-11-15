//! Unleash the Inferno: "Unleash the Inferno deals 7 damage to target creature or planeswalker. When it deals excess
//! damage this way, destroy target artifact or enchantment an opponent controls with mana value less than or equal to
//! that amount of excess damage."

use super::*;
use crate::card::{Cmp, Filter, Pred};
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnleashTheInferno.UnleashTheInfernoEffect", apply)];

/// `damageWithExcess(7)` to the target still on the battlefield; with excess, `fireReflexiveTriggeredAbility`:
/// destroy target artifact or enchantment an opponent controls with mana value less than the excess plus one
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let excess = super::damage_with_excess_effect::damage_with_excess(game, cx.source_id(), o, 7);
    if excess < 1 {
        return Some(());
    }
    let filter = Filter(vec![
        Pred::Or { xs: vec![Pred::Type { t: CardType::Artifact }, Pred::Type { t: CardType::Enchantment }] },
        Pred::Controller { who: "opponent".into() },
        Pred::Mv { cmp: Cmp::FewerThan, n: excess + 1 },
    ]);
    let target = TargetDef::TargetPermanent { filter, min: 1, max: 1, distribute: None, x_count: false, count: None };
    let delayed = Delayed {
        rule: format!(
            "destroy target artifact or enchantment an opponent controls with mana value less than or equal to {excess}."
        ),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DestroyTargetEffect],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
