//! Aetherborn Marauder: "Flying, lifelink. When Aetherborn Marauder enters, move any number of +1/+1 counters from
//! other permanents you control onto Aetherborn Marauder."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AetherbornMarauder.AetherbornMarauderEffect", apply)];

/// While another permanent you control has a +1/+1 counter: "Move any (more) +1/+1 counters …?", then the permanent
/// (`TargetControlledPermanent`, not targeted) and, with two or more on it, how many ("Choose how many +1/+1 counters
/// to move"); those come off it and go on this
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    if game.obj(me).zone != Zone::Battlefield {
        return Some(());
    }
    let mut first = true;
    loop {
        let from: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| *o != me && game.obj(*o).controller == you && game.obj(*o).counter("+1/+1") > 0)
            .collect();
        if from.is_empty() {
            break;
        }
        let message = format!(
            "Move {} +1/+1 counters from other permanents you control to {}?",
            if first { "any" } else { "more" },
            log::object(game, me)
        );
        if !choose_use(game, d, you, &message)? {
            break;
        }
        first = false;
        let chosen =
            choose_cards(game, d, you, &from, &from, 1, 1, "permanent you control to remove +1/+1 counters from")?;
        let Some(&o) = chosen.first() else { break };
        let have = game.obj(o).counter("+1/+1");
        let n = if have > 1 {
            loop {
                match ask(game, d, you, &Query::Amount { min: 0, max: have }) {
                    Answer::Int(n) if (0..=have).contains(&n) => break n,
                    Answer::Stop => return None,
                    _ => {}
                }
                if d.stopped() {
                    return None;
                }
            }
        } else {
            1
        };
        if n > 0 {
            let ob = game.obj_mut(o);
            ob.add_counters("+1/+1", -n);
            ob.counters.retain(|(k, c)| k != "+1/+1" || *c > 0);
            add_counters_by(game, me, "+1/+1", n, Some(you));
        }
    }
    Some(())
}
