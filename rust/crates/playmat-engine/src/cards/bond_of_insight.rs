//! Bond of Insight: "Each player mills four cards. Return up to two instant and/or sorcery cards from your graveyard
//! to your hand. Exile Bond of Insight."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BondOfInsight.BondOfInsightEffect", apply)];

/// each player in turn order from you mills four; then `player.choose(TargetCardInYourGraveyard(0, 2, instant or
/// sorcery))` rounds, to hand together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cx.run_on(game, d, &[EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(4) }], vec![Target::Player(p)]);
        if d.stopped() {
            return None;
        }
    }
    let possible: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Instant) || game.obj(*c).is(CardType::Sorcery))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards_rounds(game, d, you, &possible, 0, 2)?;
    for c in chosen {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
