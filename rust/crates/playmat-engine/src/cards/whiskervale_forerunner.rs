//! Whiskervale Forerunner: "Valiant — Whenever this creature becomes the target of a spell or ability you control for
//! the first time each turn, look at the top five cards of your library. You may reveal a creature card with mana
//! value 3 or less from among them. You may put it onto the battlefield if it's your turn. If you don't put it onto
//! the battlefield, put it into your hand. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WhiskervaleForerunner.WhiskervaleForerunnerEffect", apply)];

/// `player.choose(top five, TargetCardInLibrary(creature card with mana value 3 or less))`: nothing chosen, all of
/// them to the bottom; else it's revealed, on your turn "Put <it> onto the battlefield?", still in the library it
/// goes to the hand, and the rest to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    let fitting: Vec<ObjId> = top
        .iter()
        .copied()
        .filter(|c| {
            let def = &game.obj(*c).def;
            def.is(CardType::Creature) && def.mana_value() < 4
        })
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &fitting, 1, 1, &rule)?;
    let Some(&card) = chosen.first() else {
        return put_on_library(game, d, you, &top, false, false).then_some(());
    };
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    if game.active == Some(you) {
        let message = format!("Put {} onto the battlefield?", game.obj(card).def.name);
        if choose_use(game, d, you, &message)? {
            put_onto_battlefield(game, d, card, you);
            if d.stopped() {
                return None;
            }
        }
    }
    if game.obj(card).zone == Zone::Library {
        game.move_to(card, Zone::Hand);
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false).then_some(())
}
