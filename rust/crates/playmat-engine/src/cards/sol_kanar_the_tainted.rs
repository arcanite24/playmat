//! Sol'Kanar the Tainted: "At the beginning of your end step, choose one that hasn't been chosen — • Draw a card.
//! • Each opponent loses 2 life and you gain 2 life. • Sol'Kanar the Tainted deals 3 damage to up to one other target
//! creature or planeswalker. • Exile Sol'Kanar, then return it to the battlefield under an opponent's control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SolKanarTheTainted.SolKanarTheTaintedEffect", apply)];

/// Sol'Kanar (if it's still that permanent) is exiled; a token stays gone. Otherwise its controller chooses an
/// opponent (`TargetOpponent(true)`, not targeted, asked even with one) and the card enters under that player's
/// control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return None;
    }
    let you = cx.controller();
    let id = cx.source_id();
    game.move_to(id, Zone::Exile);
    if game.obj(id).token || game.obj(id).zone != Zone::Exile {
        return Some(());
    }
    let opponents: Vec<Seat> = (1..game.seats())
        .map(|k| (you + k) % game.seats())
        .filter(|p| Game::whose("opponent", you, *p) && !game.player(*p).lost)
        .collect();
    if opponents.is_empty() {
        return Some(());
    }
    // (`HumanPlayer.choose(Target)` asks even with one opponent: `tryToAutoChoose` doesn't pick a player who isn't
    // you)
    let opponent = loop {
        match ask(game, d, you, &Query::Target { players: opponents.clone(), objs: Vec::new(), required: true }) {
            Answer::Player(p) if opponents.contains(&p) => break p,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    enter_battlefield(game, d, id, opponent, false);
    Some(())
}
