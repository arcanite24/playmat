//! Nexus of Becoming: "At the beginning of combat on your turn, draw a card. Then you may exile an artifact or
//! creature card from your hand. If you do, create a token that's a copy of the exiled card, except it's a 3/3 Golem
//! artifact creature in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NexusOfBecoming.NexusOfBecomingEffect", apply)];

/// `TargetCardInHand(0, 1, FILTER_CARD_ARTIFACT_OR_CREATURE)` chosen; that card exiled and a token copy of it made
/// (`CreateTokenCopyTargetEffect`: an artifact, a creature too, 3/3, a Golem too)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let options: Vec<ObjId> = hand
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Artifact) || game.obj(*c).is(CardType::Creature))
        .collect();
    // `HumanPlayer.choose` from the hand: asked unless the hand is empty, even with nothing there to pick
    if hand.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &hand, &options, 0, 1, &rule)?.first().copied() else {
        return Some(());
    };
    game.move_to(card, Zone::Exile);
    let e = EffectDef::CreateTokenCopyTargetEffect {
        number: Some(1),
        becomes_artifact: true,
        not_legendary: false,
        add_types: vec![CardType::Creature],
        haste: false,
        flying: false,
        power: Some(3),
        toughness: Some(3),
        only_subtype: None,
        add_subtypes: vec!["Golem".into()],
        only_color: Vec::new(),
        abilities: Vec::new(),
        legendary: false,
        tapped: false,
        attacking: false,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(card)]);
    Some(())
}
