//! Miming Slime: "Create an X/X green Ooze creature token, where X is the greatest power among creatures you
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MimingSlime.MimingSlimeCreateTokenEffect", apply)
    .effects(&["new CreateTokenEffect(new OozeToken())"])];

/// `GreatestAmongPermanentsValue.POWER_CONTROLLED_CREATURES` as it resolves (0 with none)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = greatest_power(game, cx.controller());
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { token, .. } = &mut e {
        token.power = x;
        token.toughness = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}

/// the greatest power among the creatures `you` control, 0 with none
pub(super) fn greatest_power(game: &Game, you: Seat) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).0)
        .max()
        .unwrap_or(0)
        .max(0)
}
