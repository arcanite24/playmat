//! Winota, Joiner of Forces: "Whenever a non-Human creature you control attacks, look at the top six cards of your
//! library. You may put a Human creature card from among them onto the battlefield tapped and attacking. It gains
//! indestructible until end of turn. Put the rest of the cards on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WinotaJoinerOfForces.WinotaJoinerOfForcesEffect", apply)
    .effects(&["new GainAbilityTargetEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)"])];

/// `player.choose(top six, TargetCardInLibrary(0, 1, Human creature card))`; the one chosen onto the battlefield
/// tapped, then attacking (`addAttackingCreature`, its defender asked when there's a choice) with indestructible
/// until end of turn; the rest to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    let humans: Vec<ObjId> = top
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Creature) && game.has_subtype(*c, "Human"))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &humans, 0, 1, &rule)?;
    if let Some(&card) = chosen.first() {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
        if game.obj(card).zone == Zone::Battlefield {
            let defender = game.opponent(you);
            let pw = choose_defender(game, d, you, defender)?;
            let o = game.obj_mut(card);
            o.attacking = Some(defender);
            o.attacking_pw = pw;
            join_combat(game, card);
            cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(card)]);
            if d.stopped() {
                return None;
            }
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false).then_some(())
}
