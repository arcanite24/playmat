//! Guild Summit: "When Guild Summit enters, you may tap any number of untapped Gates you control. Draw a card for
//! each Gate tapped this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GuildSummit.GuildSummitEffect", apply)];

/// `TargetPermanent(0, MAX, untapped Gates you control, true).choose(Tap, …)`: rounds, then the taps and the draws;
/// asked with no Gates too (`canChoose` of zero targets holds)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Gate") && !game.obj(*o).tapped)
        .collect();
    let chosen = choose_in_rounds(game, d, you, &possible, 0, possible.len(), "tap")?;
    let mut n = 0;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield && !game.obj(o).tapped {
            tap(game, o);
            n += 1;
        }
    }
    if n > 0 {
        draw(game, you, n);
    }
    Some(())
}
