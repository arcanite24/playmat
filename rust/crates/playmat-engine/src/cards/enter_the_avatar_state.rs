//! Enter the Avatar State: "Until end of turn, target creature you control becomes an Avatar in addition to its
//! other types and gains flying, first strike, lifelink, and hexproof."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EnterTheAvatarState.AddCardSubTypeTargetEffect", apply)
    .args(&["one_of:SubType.AVATAR", "one_of:Duration.EndOfTurn"])];

/// `AddCardSubTypeTargetEffect(SubType.AVATAR, Duration.EndOfTurn)`: the target is an Avatar until end of turn
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(o).zcc;
    game.subtypes_until_eot.push((o, zcc, "Avatar".into()));
    Some(())
}
