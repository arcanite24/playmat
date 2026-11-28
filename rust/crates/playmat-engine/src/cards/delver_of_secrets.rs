//! Delver of Secrets: "At the beginning of your upkeep, look at the top card of your library. You may reveal that
//! card. If an instant or sorcery card is revealed this way, transform Delver of Secrets."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DelverOfSecrets.DelverOfSecretsEffect", apply)];

/// `lookAtCards` (titled by the source), `chooseUse`, `revealCards`, then the source transforms if it's still the
/// permanent (`getSourcePermanentIfItStillExists`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.looked_at.push(crate::state::LookedAt {
        viewer: you,
        source: cx.source_id(),
        zcc,
        bare: false,
        cards: vec![card],
    });
    if !choose_use(game, d, you, "Reveal the top card of your library?")? {
        return Some(());
    }
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    if (game.has_type(card, CardType::Instant) || game.has_type(card, CardType::Sorcery))
        && cx.source_on_battlefield(game)
    {
        transform(game, cx.source_id());
    }
    Some(())
}
