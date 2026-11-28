//! Ichormoon Gauntlet: "Whenever you cast a noncreature spell, choose a kind of counter on target permanent. Put an
//! additional counter of that kind on that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IchormoonGauntlet.IchormoonGauntletEffect", apply)];

/// One kind of counter there: one more of it. More kinds: a required `ChoiceImpl` of their names ("Choose a
/// counter"), and one more of the chosen kind
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let kinds: Vec<String> =
        game.obj(o).counters.iter().filter(|(_, n)| *n > 0).map(|(k, _)| k.clone()).collect();
    let kind = match kinds.len() {
        0 => return Some(()),
        1 => kinds[0].clone(),
        _ => {
            let q = Query::Choice { message: "Choose a counter".into(), choices: kinds.clone() };
            loop {
                match ask(game, d, cx.controller(), &q) {
                    Answer::Text(t) if kinds.contains(&t) => break t,
                    Answer::Stop => return None,
                    _ => {}
                }
                if d.stopped() {
                    return None;
                }
            }
        }
    };
    add_counters_by(game, o, &kind, 1, Some(cx.controller()));
    Some(())
}
