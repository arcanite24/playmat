//! Reptilian Recruiter: "Trample. When Reptilian Recruiter enters, choose target creature. If that creature's power
//! is 2 or less or if you control another Lizard, gain control of that creature until end of turn, untap it, and it
//! gains haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReptilianRecruiter.ReptilianRecruiterEffect", apply).effects(&[
    "new GainControlTargetEffect(Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HasteAbility.getInstance())",
])];

/// The target, still there, with power 2 or less or another Lizard you control (a permanent other than this one):
/// control of it until end of turn, then it untaps, then haste until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let you = cx.controller();
    let src = cx.source_id();
    let lizard =
        game.battlefield.iter().any(|p| *p != src && game.obj(*p).controller == you && game.has_subtype(*p, "Lizard"));
    if game.power_toughness(o).0 > 2 && !lizard {
        return Some(());
    }
    let t = vec![Target::Obj(o)];
    cx.run_on(game, d, &cx.effects[..1], t.clone());
    if d.stopped() {
        return None;
    }
    if game.obj(o).zone == Zone::Battlefield {
        untap_permanent(game, o);
    }
    cx.run_on(game, d, &cx.effects[1..], t);
    (!d.stopped()).then_some(())
}
