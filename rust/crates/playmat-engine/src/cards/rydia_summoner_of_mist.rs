//! Rydia, Summoner of Mist: "Summon — {X}, {T}: Return target Saga card with mana value X from your graveyard to
//! the battlefield with a finality counter on it. It gains haste until end of turn. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RydiaSummonerOfMist.RydiaSummonerOfMistEffect", apply).effects(&[
        "new ReturnFromGraveyardToBattlefieldWithCounterTargetEffect(CounterType.FINALITY.createInstance())",
        "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)",
    ])];

/// `super.apply` (the return with the counter), then haste until end of turn for the permanent it became
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0);
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    if let Some(c) = card.filter(|c| game.obj(*c).zone == Zone::Battlefield) {
        cx.run_on(game, d, &cx.effects[1..2], vec![Target::Obj(c)]);
    }
    Some(())
}
