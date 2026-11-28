//! Waking the Trolls: "(As this Saga enters and after your draw step, add a lore counter.) I — Destroy target land.
//! II — Put target land card from a graveyard onto the battlefield under your control. III — Choose target
//! opponent. If they control fewer lands than you, create a number of 4/4 green Troll Warrior creature tokens with
//! trample equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("WakingTheTrolls.WakingTheTrollsEffect", apply).tokens(&["TrollWarriorToken"])];

/// The lands you control and the lands the target opponent controls; fewer of theirs: that many Troll Warriors
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(Target::Player(them)) = cx.item.targets.first().and_then(|ts| ts.first()).copied() else {
        return Some(());
    };
    let lands = |g: &Game, p: Seat| {
        g.battlefield.iter().filter(|o| g.obj(**o).controller == p && g.has_type(**o, CardType::Land)).count() as i32
    };
    let (mine, theirs) = (lands(game, you), lands(game, them));
    if mine <= theirs {
        return Some(());
    }
    cx.create_tokens(game, d, &cx.tokens[0], mine - theirs);
    (!d.stopped()).then_some(())
}
