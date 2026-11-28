//! General's Enforcer: "{2}{W}{B}: Exile target card from a graveyard. If it was a creature card, create a 1/1 white
//! Human Soldier creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GeneralsEnforcer.GeneralsEnforcerEffect", apply)
    .effects(&["new CreateTokenEffect(new HumanSoldierToken())"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    let creature = game.has_type(card, CardType::Creature);
    game.move_to(card, Zone::Exile);
    if creature && game.obj(card).zone == Zone::Exile {
        cx.run_on(game, d, &cx.effects[..1], Vec::new());
    }
    Some(())
}
