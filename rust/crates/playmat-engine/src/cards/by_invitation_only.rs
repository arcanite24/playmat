//! By Invitation Only: "Choose a number between 0 and 13. Each player sacrifices that many creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ByInvitationOnly.ByInvitationOnlyEffect", apply)];

/// `Player.getAmount(0, 13)`, then `SacrificeAllEffect(number, creatures)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: 13 }) {
            Answer::Int(n) if (0..=13).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "SacrificeAllEffect", "amount": n, "filter": [{"p": "type", "t": "CREATURE"}]
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}
