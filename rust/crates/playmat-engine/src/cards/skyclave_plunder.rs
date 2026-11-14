//! Skyclave Plunder: "Look at the top X cards of your library, where X is three plus the number of creatures in your
//! party. Put three of those cards into your hand and the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkyclavePlunder.SkyclavePlunderEffect", apply)];

/// `choose(cards, TargetCardInLibrary(n, n))`, n the fewer of three and the cards looked at
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    let x = 3 + game.party_size(&mine);
    let cards: Vec<ObjId> = game.player(you).library.iter().take(x.max(0) as usize).copied().collect();
    let n = cards.len().min(3);
    let rule = item_rule(game, cx.item);
    let chosen = if n == 0 { Vec::new() } else { choose_cards(game, d, you, &cards, &cards, n, n, &rule)? };
    let rest: Vec<ObjId> = cards.iter().copied().filter(|c| !chosen.contains(c)).collect();
    for c in &chosen {
        game.move_to(*c, Zone::Hand);
    }
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
