//! Tyvar, Jubilant Brawler: "You may activate abilities of creatures you control as though those creatures had
//! haste. … −2: Mill three cards, then you may return a creature card with mana value 2 or less from your graveyard
//! to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::as_though("TyvarJubilantBrawler.TyvarJubilantBrawlerHasteEffect", AsThough::Haste(haste)),
    Handler::effect("TyvarJubilantBrawler.TyvarJubilantBrawlerReturnEffect", apply),
];

/// creatures you control
fn haste(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.obj(id).zone == Zone::Battlefield
        && game.has_type(id, CardType::Creature)
        && game.obj(id).controller == cx.controller
}

/// up to one creature card with mana value 2 or less from your graveyard, chosen (not targeted), onto the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let graveyard = game.player(you).graveyard.clone();
    let choices: Vec<ObjId> = graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Creature) && game.obj(*c).def.mana_value() < 3)
        .collect();
    let rule = item_rule(game, cx.item);
    let picked = choose_cards(game, d, you, &graveyard, &choices, 0, 1, &rule)?;
    if let Some(c) = picked.first().copied() {
        put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield);
    }
    Some(())
}
