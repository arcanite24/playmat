//! Tony Stark // The Invincible Iron Man: "At the beginning of combat on your turn, you may put an artifact card from
//! your hand onto the battlefield. If it's an Equipment, attach it to The Invincible Iron Man."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TonyStark.TheInvincibleIronManEffect", apply)];

/// With an artifact card in hand, "Put an artifact card from your hand onto the battlefield?", the card
/// (`TargetCardInHand(FILTER_CARD_ARTIFACT_AN)`), onto the battlefield; an Equipment is attached to the source while
/// it's still the same permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let artifacts: Vec<ObjId> =
        game.player(you).hand.iter().copied().filter(|c| game.has_type(*c, CardType::Artifact)).collect();
    if artifacts.is_empty() || !choose_use(game, d, you, "Put an artifact card from your hand onto the battlefield?")? {
        return Some(());
    }
    let hand = game.player(you).hand.clone();
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &hand, &artifacts, 1, 1, &rule)?.first() else { return Some(()) };
    put_onto_battlefield(game, d, card, you);
    if d.stopped() {
        return None;
    }
    let (src, zcc) = cx.source;
    let here = |g: &Game, o: ObjId, z: u32| g.obj(o).zone == Zone::Battlefield && g.obj(o).zcc == z;
    if game.obj(card).zone == Zone::Battlefield && here(game, src, zcc) && game.has_subtype(card, "Equipment") {
        let at = game.obj(src).zcc;
        game.attach_to(card, src, at);
    }
    Some(())
}
