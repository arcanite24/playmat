//! Bronze Cudgels: "{2}: Until end of turn, equipped creature gets +X/+0, where X is the number of times this
//! ability has resolved this turn. Equip {1}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BronzeCudgels.BronzeCudgelsEffect", apply)];

/// the equipped creature (the source, or as it last was) gets +X/+0 until end of turn, X the resolutions of this
/// ability this turn with the source's zone change count now (`AbilityResolvedWatcher`, this one counted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let Some(equipped) = game.attached(src) else { return Some(()) };
    if game.obj(equipped).zone != Zone::Battlefield {
        return Some(());
    }
    let Some((index, _)) = cx.item.ability else { return Some(()) };
    let (turn, now) = (game.turn, game.obj(src).zcc);
    let x = game.abilities_resolved.iter().filter(|r| r.0 == src && r.1 == index && r.2 == now && r.3 == turn).count();
    if x < 1 {
        return Some(());
    }
    let e = EffectDef::BoostTargetEffect {
        power: crate::card::Amount::Fixed(x as i32),
        toughness: crate::card::Amount::Fixed(0),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(equipped)]);
    Some(())
}
