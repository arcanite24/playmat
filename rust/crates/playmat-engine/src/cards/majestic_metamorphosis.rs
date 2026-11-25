//! Majestic Metamorphosis: "Until end of turn, target artifact or creature becomes a 4/4 Angel artifact creature and
//! gains flying. Draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MajesticMetamorphosis.MajesticMetamorphosisEffect", apply).effects(&[
        r#"new BecomesCreatureTargetEffect(new CreatureToken(4,4,"4/4 Angel artifact creature",SubType.ANGEL).withType(CardType.ARTIFACT).withAbility(FlyingAbility.getInstance()),false,false,Duration.EndOfTurn)"#,
    ])];

/// Its layers (4: an artifact creature Angel; 6: flying; 7b: 4/4) until end of turn, as the engine's
/// `BecomesCreatureTargetEffect` of a 4/4 Angel artifact creature token with flying makes them. (Java removes every
/// other card type and subtype; the engine's keeps the target's other types: the same for an artifact or a
/// creature without other types.)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    (!d.stopped()).then_some(())
}
