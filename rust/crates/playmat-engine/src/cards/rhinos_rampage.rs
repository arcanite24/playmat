//! Rhino's Rampage: "Target creature you control gets +1/+0 until end of turn. It fights target creature an opponent
//! controls. When excess damage is dealt to the creature an opponent controls this way, destroy up to one target
//! noncreature artifact with mana value 3 or less."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RhinosRampage.RhinosRampageEffect", apply)];

/// Both targets still permanents (`EachTargetPointer`): `fightWithExcess`, the second dealt the first's power first,
/// then only lethal damage of the first's power to the second; excess, a reflexive "destroy up to one target
/// noncreature artifact with mana value 3 or less"
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let on = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield;
    let (Some(a), Some(b)) = (cx.obj(0).filter(|o| on(game, *o)), cx.obj(1).filter(|o| on(game, *o))) else {
        return Some(());
    };
    let pb = game.power_toughness(b).0;
    let pa = game.power_toughness(a).0;
    deal_damage(game, b, Target::Obj(a), pb, false);
    let excess = super::damage_with_excess_effect::damage_with_excess(game, a, b, pa);
    if excess < 1 {
        return Some(());
    }
    let target: TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetPermanent",
        "filter": [{"p": "type", "t": "ARTIFACT"}, {"p": "not", "x": {"p": "type", "t": "CREATURE"}},
                   {"p": "mv", "cmp": "OR_LESS", "n": 3}],
        "min": 0, "max": 1
    }))
    .ok()?;
    let delayed = Delayed {
        rule: "Destroy up to one target noncreature artifact with mana value 3 or less.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DestroyTargetEffect],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
