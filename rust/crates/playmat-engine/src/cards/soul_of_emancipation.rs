//! Soul of Emancipation: "When Soul of Emancipation enters, destroy up to three other target nonland permanents. For
//! each of those permanents, its controller creates a 3/3 white Angel creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SoulOfEmancipation.SoulOfEmancipationEffect", apply).tokens(&["Angel33Token"])];

/// Each controller's count as the targets are before the destroys (destroyed or not), then their Angels, by seat
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let targets: Vec<ObjId> =
        cx.objs(0).iter().copied().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    if targets.is_empty() {
        return Some(());
    }
    let mut counts = vec![0; game.seats()];
    for o in &targets {
        counts[game.obj(*o).controller] += 1;
    }
    cx.run_on(game, d, &[EffectDef::DestroyTargetEffect], targets.iter().map(|o| Target::Obj(*o)).collect());
    if d.stopped() {
        return None;
    }
    for (p, n) in counts.into_iter().enumerate() {
        if n > 0 {
            let e = EffectDef::CreateTokenEffect {
                token: cx.tokens[0].clone(),
                amount: Amount::Fixed(n),
                tapped_and_attacking: false,
                tapped: false,
                attacking: false,
                sacrifice_at_end_of_combat: false,
                sacrifice_at_next_end_step: false,
                haste_until_end_of_turn: false,
                also: None,
                attach_source: None,
            };
            cx.run_as(game, d, &[e], p);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
