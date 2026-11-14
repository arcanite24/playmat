//! Mirror Mockery: "Enchant creature. Whenever enchanted creature attacks, you may create a token that's a copy of
//! that creature. Exile that token at end of combat."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MirrorMockery.MirrorMockeryEffect", apply)];

/// The creature the Aura is (or last was) attached to, copied as it is (or last was); each token exiled by an
/// `AtTheEndOfCombatDelayedTriggeredAbility`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let o = game.obj(src);
    let Some((enchanted, _)) = o.attached_to.or(o.lki_attached_to) else { return Some(()) };
    let made = token_copy_modified(game, d, enchanted, you, 1, &|_: &mut CardDef| {});
    if d.stopped() {
        return None;
    }
    for t in made {
        if game.obj(t).zone != Zone::Battlefield {
            continue;
        }
        let delayed = Delayed {
            rule: "At end of combat, exile that permanent.".into(),
            sacrifice: Vec::new(),
            warp_exile: Vec::new(),
            effects: vec![EffectDef::ExileTargetEffect],
            source_zcc: cx.source.1,
            fixed: vec![(t, game.obj(t).zcc)],
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_end_of_combat.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, you) });
    }
    Some(())
}
