//! Mardu Siegebreaker: "When this creature enters, exile up to one other target creature you control until this
//! creature leaves the battlefield. Whenever this creature attacks, for each opponent, create a tapped token that's a
//! copy of the exiled card attacking that opponent. At the beginning of your end step, sacrifice those tokens."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MarduSiegebreaker.MarduSiegebreakerEffect", apply)];

/// The card (a token ceased to exist) in the source's exile zone (`ExileUntilSourceLeavesEffect`'s, for this object); for each opponent a token
/// copy of it (`CopyTokenFunction.createTokenCopy`), tapped and attacking that opponent (no defender to choose), then
/// "at the beginning of your next end step, sacrifice" them all
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (src, zcc) = cx.source;
    let exiled: Vec<ObjId> = game
        .exiled_until_leaves
        .iter()
        .filter(|(s, z, c)| *s == src && *z == zcc && game.obj(*c).zone == Zone::Exile && !game.obj(*c).token)
        .map(|(_, _, c)| *c)
        .collect();
    let card = match exiled.as_slice() {
        [] => return Some(()),
        [c] => *c,
        _ => {
            let rule = item_rule(game, cx.item);
            *choose_cards(game, d, you, &exiled, &exiled, 1, 1, &rule)?.first()?
        }
    };
    let opponents: Vec<Seat> =
        (1..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let mut made = Vec::new();
    for opponent in opponents {
        let tokens = token_copy_modified(game, d, card, you, 1, &|_: &mut CardDef| {});
        if d.stopped() {
            return None;
        }
        for t in tokens {
            if game.obj(t).zone != Zone::Battlefield {
                continue;
            }
            game.obj_mut(t).tapped = true;
            if game.active == Some(you) {
                let o = game.obj_mut(t);
                o.attacking = Some(opponent);
                o.attacking_pw = None;
                join_combat(game, t);
            }
            made.push(t);
        }
    }
    let delayed = Delayed {
        rule: "At the beginning of your next end step, that permanent's controller sacrifices it.".into(),
        sacrifice: made.iter().map(|t| (*t, game.obj(*t).zcc)).collect(),
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, you) });
    Some(())
}
