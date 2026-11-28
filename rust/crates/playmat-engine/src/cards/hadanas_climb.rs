//! Hadana's Climb // Winged Temple of Orazca: "{1}{G}{U}, {T}: Target creature you control gains flying and gets
//! +X/+X until end of turn, where X is its power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HadanasClimb.WingedTempleOfOrazcaEffect", apply)];

/// X is the creature's power as this resolves; the boost, then flying
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || !game.has_type(o, CardType::Creature) {
        return Some(());
    }
    let power = game.power_toughness(o).0;
    boost(game, o, power, power, crate::card::Duration::EndOfTurn);
    grant(game, o, &AbilityDef::FlyingAbility);
    Some(())
}
