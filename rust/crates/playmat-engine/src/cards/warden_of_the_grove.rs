//! Warden of the Grove: "Whenever another nontoken creature you control enters, it endures X, where X is the number
//! of counters on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WardenOfTheGrove.WardenOfTheGroveEffect", apply).tokens(&[
    "NoFlyingSpiritWhite11Token",
    "NoFlyingSpiritWhite22Token",
    "NoFlyingSpiritWhite33Token",
])];

/// `EndureSourceEffect.doEndure` on the creature that entered, X the counters on Warden (as it last was if it's
/// gone): its controller is asked counters or a Spirit while it's still that permanent; the Spirit is yours
/// (`Token.putOntoBattlefield(1, game, source)`), an X/X `SpiritXXToken` past 3
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let it = cx.item.entering?;
    let w = game.obj(cx.source_id());
    let counters = if cx.source_on_battlefield(game) { &w.counters } else { &w.lki_counters };
    let n: i32 = counters.iter().map(|(_, k)| (*k).max(0)).sum();
    if n < 1 {
        return Some(());
    }
    let p = game.obj(it).controller;
    let here = game.obj(it).zone == Zone::Battlefield;
    let counters = here && {
        let amount = match n {
            1 => "a".to_string(),
            2 => "two".to_string(),
            3 => "three".to_string(),
            _ => n.to_string(),
        };
        let message = format!(
            "Put {amount} +1/+1 counter{} on {} or create a {n} {n}/{n} Spirit token?",
            if n > 1 { "s" } else { "" },
            game.obj(it).name()
        );
        choose_use(game, d, p, &message)?
    };
    if counters {
        put_counters(game, it, "+1/+1", n, cx.controller());
    } else {
        let mut t = cx.tokens[(n.min(3) - 1) as usize].clone();
        if t.power != n {
            t.power = n;
            t.toughness = n;
        }
        cx.run(game, d, &[token_effect(&t, 1)]);
    }
    Some(())
}
