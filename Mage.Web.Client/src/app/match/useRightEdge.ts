import { useCallback, useRef, useState } from 'react';

/**
 * The right edge of an element in its offset parent's unscaled pixels (stage pixels, inside the stage), kept current
 * as the element grows or shrinks: a ref callback and the edge (0 until it mounts).
 */
export function useRightEdge(): [(element: HTMLElement | null) => void, number] {
  const [edge, setEdge] = useState(0);
  const observer = useRef<ResizeObserver | null>(null);
  const ref = useCallback((element: HTMLElement | null) => {
    observer.current?.disconnect();
    observer.current = null;
    if (!element) {
      setEdge(0);
      return;
    }
    const measure = () => setEdge(Math.ceil(element.offsetLeft + element.offsetWidth));
    measure();
    observer.current = new ResizeObserver(measure);
    observer.current.observe(element);
  }, []);
  return [ref, edge];
}
