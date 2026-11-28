//! Red Hulk: "Enrage — Whenever Red Hulk is dealt damage, put a +1/+1 counter on him. When you do, he deals damage
//! equal to the number of +1/+1 counters on him to any other target."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RedHulk.RedHulkEffect", apply)
    .effects(&["new DamageTargetEffect(new CountersSourceCount(CounterType.P1P1))"])];

/// A +1/+1 counter on him while he's on the battlefield, then the reflexive trigger: damage equal to his +1/+1
/// counters to any other target (`TargetPermanentOrPlayer(filter)`, the permanent not him)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target: crate::card::TargetDef =
        serde_json::from_str(r#"{"class": "TargetAnyTarget", "another": true}"#).expect("any other target");
    let e = EffectDef::AddCounterSourceReflexiveEffect {
        counter: crate::card::CounterDef { name: "+1/+1".into(), amount: 1 },
        rule: "he deals damage equal to the number of +1/+1 counters on him to any other target.".into(),
        condition: None,
        targets: vec![target],
        effects: cx.effects[..1].to_vec(),
    };
    cx.run(game, d, &[e]);
    Some(())
}
