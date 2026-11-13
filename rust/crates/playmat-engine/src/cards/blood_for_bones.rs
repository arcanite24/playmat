//! Blood for Bones: "Return a creature card from your graveyard to the battlefield, then return another creature
//! card from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BloodForBones.BloodForBonesEffect", apply)];

fn creature_cards(game: &Game, you: Seat) -> Vec<ObjId> {
    game.player(you).graveyard.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect()
}

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    // Java's `getRule(true)` has the additional cost's text too: "sacrifice" keeps a lone card of yours from being
    // picked by itself (`TargetImpl.tryToAutoChoose`)
    let rule = format!("As an additional cost to cast this spell, sacrifice a creature. {}", item_rule(game, cx.item));
    for zone in [Zone::Battlefield, Zone::Hand] {
        let creatures = creature_cards(game, you);
        if creatures.is_empty() {
            return Some(());
        }
        let graveyard = game.player(you).graveyard.clone();
        let picked = choose_cards(game, d, you, &graveyard, &creatures, 1, 1, &rule)?;
        if let Some(c) = picked.first().copied() {
            if zone == Zone::Battlefield {
                put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield);
            } else {
                game.move_to(c, Zone::Hand);
            }
        }
    }
    Some(())
}
