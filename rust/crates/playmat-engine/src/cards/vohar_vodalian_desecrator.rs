//! Vohar, Vodalian Desecrator: "{T}: Draw a card, then discard a card. If you discarded an instant or sorcery card
//! this way, each opponent loses 1 life and you gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("VoharVodalianDesecrator.VoharVodalianDesecratorEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    let spell = |g: &Game, c: ObjId| g.has_type(c, CardType::Instant) || g.has_type(c, CardType::Sorcery);
    if discarded.iter().any(|c| spell(game, *c)) {
        let opponent = game.opponent(you);
        lose_life(game, opponent, 1);
        gain_life(game, you, 1);
    }
    Some(())
}
