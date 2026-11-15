//! Diviner's Lockbox: "{1}, {T}: Choose a card name, then reveal the top card of your library. If that card has the
//! chosen name, sacrifice Diviner's Lockbox and draw three cards. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DivinersLockbox.DivinersLockboxEffect", apply)];

/// `TypeOfName.ALL.getChoice` ("Choose a card name"), the top card revealed, then on a match the Lockbox sacrificed
/// (if it's still the permanent that activated) and three cards drawn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let q = Query::Choice { message: "Choose a card name".into(), choices: Vec::new() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let top = game.player(you).library.first().copied();
    if let Some(card) = top {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
    }
    if top.is_some_and(|c| game.obj(c).name() == chosen) {
        if cx.source_on_battlefield(game) {
            sacrifice(game, cx.source_id());
        }
        draw(game, you, 3);
    }
    Some(())
}
