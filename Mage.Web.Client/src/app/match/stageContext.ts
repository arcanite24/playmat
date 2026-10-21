import { createContext, useCallback, useContext } from 'react';

/** The reference layout: 1080 tall, and at least 1920 wide. */
export const STAGE_WIDTH = 1920;
export const STAGE_HEIGHT = 1080;
/** Wider windows widen the stage (the battlefield grows) up to this; beyond it the mat fills the sides. */
export const MAX_STAGE_WIDTH = 2640;
/** Landscape windows squarer than 16:9 (16:10, 4:3) heighten the stage instead, up to this: the rows grow. */
export const MAX_STAGE_HEIGHT = 1440;
/**
 * Portrait windows (a tablet held upright) get the tall layout: a narrower stage, 1080 wide, that grows downward
 * with the window, between these heights. Both players' rows span its width; the seats and controls move to bands
 * above and below them.
 */
export const TALL_STAGE_WIDTH = 1080;
export const MIN_TALL_HEIGHT = 1440;
export const MAX_TALL_HEIGHT = 1920;

export type StageLayout = 'wide' | 'tall';

export interface StageContextValue {
  scale: number;
  element: HTMLDivElement | null;
  /** the stage's current width in stage pixels: STAGE_WIDTH to MAX_STAGE_WIDTH wide, TALL_STAGE_WIDTH tall */
  width: number;
  /** the stage's current height in stage pixels: STAGE_HEIGHT to MAX_STAGE_HEIGHT wide, MIN_TALL_HEIGHT to MAX_TALL_HEIGHT tall */
  height: number;
  layout: StageLayout;
}

export const DEFAULT_STAGE: StageContextValue = { scale: 1, element: null, width: STAGE_WIDTH, height: STAGE_HEIGHT, layout: 'wide' };

export const StageContext = createContext<StageContextValue>(DEFAULT_STAGE);

/** The stage that fits a window of this size: its layout, size in stage pixels and scale to the window. */
export function fitStage(windowWidth: number, windowHeight: number): Pick<StageContextValue, 'layout' | 'width' | 'height' | 'scale'> {
  if (windowHeight > windowWidth) {
    const height = Math.round(Math.min(MAX_TALL_HEIGHT, Math.max(MIN_TALL_HEIGHT, TALL_STAGE_WIDTH * (windowHeight / windowWidth))));
    return { layout: 'tall', width: TALL_STAGE_WIDTH, height, scale: Math.min(windowWidth / TALL_STAGE_WIDTH, windowHeight / height) };
  }
  const width = Math.round(Math.min(MAX_STAGE_WIDTH, Math.max(STAGE_WIDTH, STAGE_HEIGHT * (windowWidth / windowHeight))));
  const height = Math.round(Math.min(MAX_STAGE_HEIGHT, Math.max(STAGE_HEIGHT, STAGE_WIDTH * (windowHeight / windowWidth))));
  return { layout: 'wide', width, height, scale: Math.min(windowWidth / width, windowHeight / height) };
}

export function useStage() {
  return useContext(StageContext);
}

/** Converts a viewport point to stage coordinates. */
export function toStagePoint(stage: StageContextValue, clientX: number, clientY: number) {
  const rect = stage.element?.getBoundingClientRect();
  if (!rect) return { x: clientX, y: clientY };
  return { x: (clientX - rect.left) / stage.scale, y: (clientY - rect.top) / stage.scale };
}

/** Center of a board object (card, player plate, stack item) in stage coordinates. */
export function useObjectCenter() {
  const stage = useStage();
  // stable while the stage keeps its size, so effects can depend on it
  return useCallback((id: string): { x: number; y: number } | null => {
    const element = stage.element?.querySelector(`[data-object-id="${CSS.escape(id)}"]`);
    if (!element) return null;
    const box = element.getBoundingClientRect();
    return toStagePoint(stage, box.left + box.width / 2, box.top + box.height / 2);
  }, [stage]);
}
