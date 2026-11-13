//! God-Pharaoh's Gift: "At the beginning of combat on your turn, you may exile a creature card from your graveyard.
//! If you do, create a token that's a copy of that card, except it's a 4/4 black Zombie. It gains haste until end of
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GodPharaohsGift.GodPharaohsGiftEffect", apply)];

/// `TargetCardInYourGraveyard(0, 1)` in one `choose` (asked even with none), then the copy: 4/4, black, Zombie in
/// place of its creature types, and haste until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?.first() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    let mods = |def: &mut CardDef| {
        def.remove_pt_cda();
        def.power = 4;
        def.toughness = 4;
        def.colors = vec![Color::B];
        def.subtypes.retain(|s| crate::card::NON_CREATURE_TYPES.contains(&s.as_str()));
        def.all_creature_types = false;
        def.subtypes.push("Zombie".into());
    };
    let made = token_copy_modified(game, d, card, you, 1, &mods);
    if d.stopped() {
        return None;
    }
    for t in made {
        if game.obj(t).zone == Zone::Battlefield {
            grant(game, t, &AbilityDef::HasteAbility);
        }
    }
    Some(())
}
