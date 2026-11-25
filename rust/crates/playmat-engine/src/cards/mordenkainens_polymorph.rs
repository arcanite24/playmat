//! Mordenkainen's Polymorph: "Until end of turn, target creature becomes a Dragon with base power and toughness 4/4
//! and gains flying."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MordenkainensPolymorph.MordenkainensPolymorphEffect", apply).effects(&[
        r#"new BecomesCreatureTargetEffect(new CreatureToken(4,4,"4/4 Dragon creature",SubType.DRAGON).withAbility(FlyingAbility.getInstance()),false,false,Duration.EndOfTurn)"#,
    ])];

/// Its layers (4: its creature types replaced by Dragon, its card types kept; 6: flying; 7b: base 4/4) until end of
/// turn, as the engine's `BecomesCreatureTargetEffect` of a 4/4 Dragon creature token with flying makes them
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    (!d.stopped()).then_some(())
}
