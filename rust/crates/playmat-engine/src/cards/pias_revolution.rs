//! Pia's Revolution: "Whenever a nontoken artifact is put into your graveyard from the battlefield, return that card
//! to your hand unless target opponent has Pia's Revolution deal 3 damage to them."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "PiasRevolution.PiasRevolutionTriggeredAbility",
        "new PutIntoGraveFromBattlefieldAllTriggeredAbility(new PiasRevolutionReturnEffect(), false, StaticFilters.FILTER_PERMANENT_ARTIFACT, false)",
        yours_nontoken,
    ),
    Handler::effect("PiasRevolution.PiasRevolutionReturnEffect", apply),
];

/// the artifact that died was a nontoken one you own (`TokenPredicate.FALSE`, `TargetController.YOU`'s owner
/// predicate)
fn yours_nontoken(game: &Game, t: &StackItem) -> bool {
    t.died.is_some_and(|(c, _)| {
        let o = game.obj(c);
        !o.token && o.owner == t.controller
    })
}

/// The card still in the graveyard: the target opponent chooses whether Pia's Revolution deals them 3 damage;
/// if not, the card returns to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((c, _)) = cx.item.died else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let opponent = cx.player(0);
    if let Some(p) = opponent {
        if choose_use(game, d, p, "Have Pia's Revolution deal 3 damage to you to prevent return to hand effect?")? {
            deal_damage(game, cx.source_id(), Target::Player(p), 3, false);
            return Some(());
        }
    }
    game.move_to(c, Zone::Hand);
    Some(())
}
