//! Destiny Spinner: "{3}{G}: Target land you control becomes an X/X Elemental creature with trample and haste until
//! end of turn, where X is the number of enchantments you control. It's still a land."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DestinySpinner.DestinySpinnerEffect", apply).effects(&[
    "new BecomesCreatureTargetEffect(new CreatureToken(0,0,\"\").withSubType(SubType.ELEMENTAL).withAbility(TrampleAbility.getInstance()).withAbility(HasteAbility.getInstance()), false, true, Duration.EndOfTurn)",
])];

/// X counted as it resolves (`DestinySpinnerCount`: the enchantments you control), fixed from then on
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Enchantment))
        .count() as i32;
    let mut e = cx.effects[0].clone();
    if let EffectDef::BecomesCreatureTargetEffect { token, .. } = &mut e {
        token.power = x;
        token.toughness = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}
