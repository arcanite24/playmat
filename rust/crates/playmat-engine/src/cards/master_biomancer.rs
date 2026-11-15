//! Master Biomancer: "Each other creature you control enters with a number of additional +1/+1 counters on it equal
//! to Master Biomancer's power and as a Mutant in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MasterBiomancer.MasterBiomancerEntersBattlefieldEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)];

/// another creature entering under your control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    id != cx.source && ev.player == cx.controller && game.has_type(id, CardType::Creature)
}

/// Master Biomancer's power in +1/+1 counters (none at 0 or less), then a Mutant for as long as it stays
/// (`AddCardSubTypeTargetEffect(MUTANT, Duration.Custom)`); nothing while Biomancer isn't on the battlefield
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let Some(id) = ev.target else { return false };
    if game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    let power = game.power_toughness(cx.source).0;
    if power > 0 {
        add_counters_by(game, id, "+1/+1", power, Some(cx.controller));
    }
    let zcc = game.obj(id).zcc;
    game.subtypes_while_there.push((id, zcc, "Mutant".into()));
    false
}
