//! Realmbreaker, the Invasion Tree: "{2}, {T}: Target opponent mills three cards. Put a land card from their
//! graveyard onto the battlefield tapped under your control. It gains "If this land would leave the battlefield,
//! exile it instead of putting it anywhere else." {10}, {T}, Sacrifice Realmbreaker, the Invasion Tree: Search your
//! library for any number of Praetor cards, put them onto the battlefield, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RealmbreakerTheInvasionTree.RealmbreakerTheInvasionTreeEffect", apply)];

/// With a land card in the opponent's graveyard: one chosen (`TargetCardInGraveyard`, not targeted), put onto the
/// battlefield tapped under your control; while it stays, it's exiled if it would leave (the same replacement as
/// unearth's, `LeaveBattlefieldExileSourceReplacementEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opp) = cx.player(0) else { return Some(()) };
    let yard = game.player(opp).graveyard.clone();
    let lands: Vec<ObjId> = yard.iter().copied().filter(|c| game.obj(*c).is(CardType::Land)).collect();
    if lands.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &yard, &lands, 1, 1, &rule)?.first() else { return Some(()) };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        game.obj_mut(card).unearthed = true;
    }
    Some(())
}
