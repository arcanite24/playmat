//! Brilliance Unleashed: "• Choose target artifact card in your graveyard. Return it to the battlefield if it's an
//! artifact creature card. Otherwise, return it to the battlefield and it's a 3/3 Robot artifact creature with
//! flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BrillianceUnleashed.BrillianceUnleashedEffect", apply).effects(&[
    "new BecomesCreatureTargetEffect(new CreatureToken(3,3,\"3/3 Robot artifact creature with flying\",SubType.ROBOT).withType(CardType.ARTIFACT).withAbility(FlyingAbility.getInstance()),false,false,Duration.WhileOnBattlefield)",
])];

/// The card onto the battlefield; a permanent that isn't a creature then becomes the Robot until it leaves
/// (`Duration.Custom`, a resolved "while on the battlefield")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = cx.obj(0)?;
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield && !game.has_type(card, CardType::Creature) {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(card)]);
    }
    Some(())
}
