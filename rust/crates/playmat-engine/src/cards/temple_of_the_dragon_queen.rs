//! Temple of the Dragon Queen: "As Temple of the Dragon Queen enters, you may reveal a Dragon card from your hand.
//! Temple of the Dragon Queen enters tapped unless you revealed a Dragon card this way or you control a Dragon. As
//! Temple of the Dragon Queen enters, choose a color. {T}: Add one mana of the chosen color."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TempleOfTheDragonQueen.TempleOfTheDragonQueenEffect", apply)];

/// With a Dragon in hand, "Reveal a Dragon card from your hand?", then `TargetCardInHand(Dragon)` chosen and
/// revealed; else (or with no Dragon revealed) a Dragon you control keeps it untapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let mut tapped = true;
    let hand = game.player(you).hand.clone();
    let dragons: Vec<ObjId> = hand.iter().copied().filter(|c| game.has_subtype(*c, "Dragon")).collect();
    if !dragons.is_empty() && choose_use(game, d, you, "Reveal a Dragon card from your hand?")? {
        let rule = item_rule(game, cx.item);
        if let Some(c) = choose_cards(game, d, you, &hand, &dragons, 1, 1, &rule)?.first().copied() {
            let zcc = game.obj(me).zcc;
            game.revealed.push(crate::state::Revealed { source: me, zcc, paren: false, cards: vec![(c, Zone::Hand)] });
            tapped = false;
        }
    }
    if tapped && game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Dragon")) {
        tapped = false;
    }
    if tapped {
        game.obj_mut(me).tapped = true;
    }
    Some(())
}
