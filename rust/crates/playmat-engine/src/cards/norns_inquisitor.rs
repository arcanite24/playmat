//! Norn's Inquisitor: "Whenever a permanent you control transforms into a Phyrexian, put a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "NornsInquisitor.NornsInquisitorTriggeredAbility",
    "new TransformsAllTriggeredAbility(new AddCountersTargetEffect(CounterType.P1P1.createInstance()), new FilterControlledPermanent(SubType.PHYREXIAN, \"Phyrexian\"), true)",
    any_batch,
)];
