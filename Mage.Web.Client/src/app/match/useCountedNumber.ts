import { useEffect, useRef, useState } from 'react';

/**
 * A number that counts to its new value instead of jumping (a life total taking a hit, #413): one step per frame
 * over a short time that grows with the change, capped so a big swing still settles quickly. A drop waits `dropDelay`
 * ms first, so a combat hit's count starts about when the swing lands. Off, it just follows.
 */
export function useCountedNumber(value: number, enabled: boolean, dropDelay = 0): number {
  const [shown, setShown] = useState(value);
  const from = useRef(value);
  useEffect(() => {
    const start = from.current;
    from.current = value;
    if (!enabled || start === value || typeof requestAnimationFrame === 'undefined') {
      setShown(value);
      return;
    }
    const duration = Math.min(700, 220 + Math.abs(value - start) * 45);
    const wait = value < start ? dropDelay : 0;
    const began = performance.now() + wait;
    let frame = requestAnimationFrame(function step(time) {
      const t = Math.max(0, Math.min(1, (time - began) / duration));
      // ease out: fast at first, settling on the value
      const eased = 1 - (1 - t) ** 3;
      setShown(Math.round(start + (value - start) * eased));
      if (t < 1) frame = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(frame);
  }, [value, enabled, dropDelay]);
  return enabled ? shown : value;
}
