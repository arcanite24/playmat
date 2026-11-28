//! The Kami War // O-Kagachi Made Manifest: "Whenever O-Kagachi Made Manifest attacks, defending player chooses a
//! nonland card in your graveyard. Return that card to your hand. O-Kagachi Made Manifest gets +X/+0 until end of
//! turn, where X is the mana value of that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheKamiWar.OKagachiMadeManifestEffect", apply)];

/// One nonland card in your graveyard: that one, no prompt; more: the defending player (the trigger's target
/// pointer) chooses (not targeted). It goes to your hand; its mana value, if above 0, is the boost
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let graveyard = game.player(you).graveyard.clone();
    let nonland: Vec<ObjId> = graveyard.iter().copied().filter(|c| !game.has_type(*c, CardType::Land)).collect();
    let card = match nonland.len() {
        0 => return Some(()),
        1 => nonland[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, p, &graveyard, &nonland, 1, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    let mv = game.obj(card).def.mana_value() as i32;
    game.move_to(card, Zone::Hand);
    if mv > 0 {
        let e = EffectDef::BoostSourceEffect {
            power: crate::card::Amount::Fixed(mv),
            toughness: crate::card::Amount::Fixed(0),
            duration: crate::card::Duration::EndOfTurn,
        };
        cx.run(game, d, &[e]);
    }
    Some(())
}
