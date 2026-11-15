//! Bile Blight: "Target creature and all other creatures with the same name as that creature get -3/-3 until end of
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BileBlight.BileBlightEffect", apply)];

/// `BoostAllEffect.init`'s affected objects: the target alone when it has no name (face down), else every permanent
/// with its name; each gets -3/-3 until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.obj(0) else { return Some(()) };
    let name = game.obj(target).name().to_string();
    let affected: Vec<Target> = if name.is_empty() {
        vec![Target::Obj(target)]
    } else {
        game.battlefield.iter().copied().filter(|o| game.obj(*o).name() == name).map(Target::Obj).collect()
    };
    let e = EffectDef::BoostTargetEffect {
        power: Amount::Fixed(-3),
        toughness: Amount::Fixed(-3),
        duration: Duration::EndOfTurn,
    };
    cx.run_on(game, d, &[e], affected);
    (!d.stopped()).then_some(())
}
