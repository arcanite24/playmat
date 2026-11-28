//! Okiba Salvage: "Return target creature or Vehicle card from your graveyard to the battlefield. Then put two +1/+1
//! counters on that permanent if you control an artifact and an enchantment."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OkibaSalvage.OkibaSalvageEffect", apply)];

/// `moveCards` onto the battlefield, then the counters on the permanent it became
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(c).zone != Zone::Battlefield {
        return Some(());
    }
    let controls =
        |t: CardType| game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_type(*o, t));
    if controls(CardType::Artifact) && controls(CardType::Enchantment) {
        add_counters_by(game, c, "+1/+1", 2, Some(you));
    }
    Some(())
}
