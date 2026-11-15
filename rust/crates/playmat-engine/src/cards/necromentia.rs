//! Necromentia: "Choose a card name other than a basic land card name. Search target opponent's graveyard, hand, and
//! library for any number of cards with that name and exile them. That player shuffles, then creates a 2/2 black
//! Zombie creature token for each card exiled from their hand this way."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("Necromentia.NecromentiaEffect", apply).tokens(&["ZombieToken"])];

/// The name `ChooseACardNameEffect` kept on the spell; the opponent's graveyard, hand and library in turn, each a
/// `TargetCard(0, n)` of the cards with that name chosen with `Player.choose` (nothing asked with none there), the
/// chosen exiled; their shuffle, then a Zombie for each card exiled from the hand, under that player's control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(name) = game.obj(cx.source_id()).chosen_name.clone() else { return Some(()) };
    let Some(p) = cx.player(0) else { return Some(()) };
    let named = |game: &Game, zone: &[ObjId]| -> Vec<ObjId> {
        if name.is_empty() {
            return Vec::new();
        }
        zone.iter().copied().filter(|c| game.obj(*c).def.name == name).collect()
    };
    let mut from_hand = 0;
    for zone in [Zone::Graveyard, Zone::Hand, Zone::Library] {
        let cards = match zone {
            Zone::Graveyard => game.player(p).graveyard.clone(),
            Zone::Hand => game.player(p).hand.clone(),
            _ => game.player(p).library.clone(),
        };
        let of_name = named(game, &cards);
        if of_name.is_empty() {
            continue;
        }
        let chosen = choose_cards(game, d, you, &cards, &of_name, 0, of_name.len(), "exile")?;
        if zone == Zone::Hand {
            from_hand = chosen.len();
        }
        for c in chosen {
            game.move_to(c, Zone::Exile);
        }
    }
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
    if from_hand > 0 {
        let e = EffectDef::CreateTokenEffect {
            token: cx.tokens[0].clone(),
            amount: Amount::Fixed(from_hand as i32),
            tapped_and_attacking: false,
            tapped: false,
            attacking: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            haste_until_end_of_turn: false,
            also: None,
            attach_source: None,
        };
        cx.run_as(game, d, &[e], p);
    }
    Some(())
}
