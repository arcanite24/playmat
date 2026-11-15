//! Enigmatic Incarnation: "At the beginning of your end step, you may sacrifice another enchantment. If you do,
//! search your library for a creature card with mana value equal to 1 plus the sacrificed enchantment's mana value,
//! put that card onto the battlefield, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("EnigmaticIncarnation.EnigmaticIncarnationEffect", apply).filters(&["filter"])];

/// With another enchantment you control: "Sacrifice an enchantment?", then one chosen (`TargetPermanent(0, 1)`,
/// not targeted), its mana value read and it sacrificed; then `SearchLibraryPutInPlayEffect` for a creature card
/// with mana value one more
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = matching(game, &cx.filters[0], you, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == you)
        .collect();
    if mine.is_empty() || !choose_use(game, d, you, "Sacrifice an enchantment?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&chosen) = choose_cards(game, d, you, &mine, &mine, 0, 1, &rule)?.first() else { return Some(()) };
    let mv = game.obj(chosen).def.mana_value() as i32;
    sacrifice(game, chosen);
    if game.obj(chosen).zone == Zone::Battlefield {
        return Some(());
    }
    let e = EffectDef::SearchLibraryEffect {
        filter: Filter(vec![Pred::Type { t: CardType::Creature }, Pred::Mv { cmp: Cmp::EqualTo, n: mv + 1 }]),
        min: 1,
        max: 1,
        to: crate::card::PutCards::Battlefield,
        optional: false,
        target_controller: false,
    };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
