//! Arni Brokenbrow: "Haste. Boast — {1}: You may change Arni Brokenbrow's base power to 1 plus the greatest power
//! among other creatures you control until end of turn. (Activate only if this creature attacked this turn and only
//! once each turn.)"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArniBrokenbrow.ArniBrokenbrowEffect", apply)];

/// `GreatestAmongPermanentsValue.POWER_OTHER_CONTROLLED_CREATURES` (0 with none) plus one; "Change base power of
/// <it> to N until end of turn?", and on a yes `SetBasePowerSourceEffect(N, EndOfTurn)`. (The engine sets base power
/// and toughness together: the toughness stays the base one it has now.)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let power = 1 + controlled_creatures(game, you)
        .into_iter()
        .filter(|c| *c != src)
        .map(|c| game.power_toughness(c).0)
        .max()
        .unwrap_or(0);
    let name = log::object(game, src);
    if !choose_use(game, d, you, &format!("Change base power of {name} to {power} until end of turn?"))? {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        let toughness = game.base_power_toughness(src).1;
        let (zcc, at) = (game.obj(src).zcc, game.timestamp());
        game.base_pt_until_eot.push((src, zcc, power, toughness, at));
    }
    Some(())
}
