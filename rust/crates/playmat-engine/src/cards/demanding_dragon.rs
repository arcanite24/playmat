//! Demanding Dragon: "When Demanding Dragon enters the battlefield, it deals 5 damage to target opponent unless that
//! player sacrifices a creature." (`DoUnlessTargetPlayerOrTargetsControllerPaysEffect(DamageTargetEffect(5),
//! SacrificeTargetCost(creature))`, this card's form of it)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DemandingDragon.DoUnlessTargetPlayerOrTargetsControllerPaysEffect", apply).args(&["effect"])];

/// With a creature, the player is asked; yes, they choose one and sacrifice it ("pays the cost"). Else the damage.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| game.obj(*b).controller == p && game.has_type(*b, CardType::Creature))
        .collect();
    if !creatures.is_empty()
        && choose_use(game, d, p, "Pay Sacrifice a creature to prevent (it deals 5 damage to target opponent)?")?
    {
        let chosen = choose_cards(game, d, p, &creatures, &creatures, 1, 1, &item_rule(game, cx.item))?;
        if let Some(c) = chosen.first().copied() {
            sacrifice(game, c);
            if log::kept(game) {
                let line = format!("{} pays the cost to prevent the effect", log::player(p));
                log::inform(game, line);
            }
            return Some(());
        }
    }
    cx.run(game, d, cx.effects);
    Some(())
}
