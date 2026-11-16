//! Hellcat, Undying Vigilante: "When Hellcat dies, return her to the battlefield under her owner's control with a
//! +1/+1 counter on her. She loses all abilities and gains haste."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("HellcatUndyingVigilante.HellcatUndyingVigilanteReturnEffect", apply).effects(&[
        "new ReturnFromGraveyardToBattlefieldWithCounterTargetEffect(CounterType.P1P1.createInstance())",
        "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.Custom)",
    ])];

/// Only while the card is in a graveyard: back with the counter, then (once it's on the battlefield) it loses all
/// abilities (`LoseAllAbilitiesTargetEffect(Duration.Custom)`: until it leaves) and gains haste for as long as it
/// stays
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(card)]);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(card).zcc;
    lose_all_abilities(game, card, card, zcc);
    cx.run_on(game, d, &cx.effects[1..], vec![Target::Obj(card)]);
    Some(())
}
