import { MIN_TALL_HEIGHT, STAGE_HEIGHT, type StageContextValue } from './stageContext';

/** Where a player's two rows of permanents sit, and how big their cards may grow. */
export interface RowsPlacement {
  left: number;
  width: number;
  frontTop: number;
  backTop: number;
  /** ideal card widths: creatures (front) and lands and other permanents (back) */
  frontIdeal: number;
  backIdeal: number;
}

/**
 * The match's zones in stage pixels, for the stage's layout. Wide (a desktop, a tablet on its side): seats on the
 * left, the stack and decision corner on the right, the battlefield between. Tall (a tablet held upright): both
 * players' rows span the stage; the opponent's seat and piles are a band along the top, yours a band above your hand
 * at the bottom, and the turn and stack sit between the two battlefields.
 */
export interface MatchLayout {
  tall: boolean;
  /** the battlefield's horizontal span */
  fieldLeft: number;
  fieldWidth: number;
  me: RowsPlacement;
  /** a single opponent's rows; several opponents share `fieldWidth` in columns */
  them: (index: number, count: number) => RowsPlacement;
  /** the line between the two halves of the table */
  seam: number;
  /** a hand card dragged above this line is played */
  playLine: number;
  /**
   * your seat: wide, anchored by its bottom-left corner (`bottom` from the stage's bottom edge), so it grows upward as
   * counters, commander damage and floating mana arrive instead of off the stage; tall, by its top-left corner
   */
  myPlate: { left: number; top?: number; bottom?: number };
  /** your piles (top-left corner) */
  myPiles: { left: number; top: number };
  /** the opponent's seat: index among `count` opponents */
  theirPlate: (index: number, count: number) => { left: number; top: number };
  theirPiles: { left: number; top: number };
  /** the centre of an opponent's hidden hand along the top edge */
  hiddenHandX: (index: number, count: number) => number;
  /** where a resolving stack object bursts (the top of the stack) */
  stackPoint: { x: number; y: number };
  /** tall: how much higher than usual the hand stands, so more of each card shows */
  handRaise: number;
  /** the hand keeps this far from the stage's right edge: wide, clear of the decision corner */
  handClearRight: number;
}

/** A hand card's width in stage pixels. */
export const HAND_CARD_WIDTH = 168;
/** The hand's fan: cards at most this far apart, the fan at most HAND_FAN + a card wide, cards at least this far apart. */
const HAND_MAX_SPREAD = HAND_CARD_WIDTH * 0.78;
const HAND_FAN = 1000;
const HAND_MIN_SPREAD = 36;

/**
 * Where the hand's fan sits along the bottom edge: centred on the stage when it can be, moved over (and fanned
 * tighter, if it must) to keep between `clearLeft` (a crowded seat) and `clearRight` (the decision corner). When even
 * the tightest fan can't fit, it keeps clear of the seat. `center` is the fan's middle, `spread` the step between cards.
 */
export function placeHand(count: number, stageWidth: number, clearLeft = 0, clearRight = 0): { center: number; spread: number; width: number } {
  const natural = Math.min(HAND_MAX_SPREAD, HAND_FAN / Math.max(1, count));
  const room = stageWidth - clearLeft - clearRight;
  const fit = count > 1 ? (room - HAND_CARD_WIDTH) / (count - 1) : natural;
  const spread = Math.max(HAND_MIN_SPREAD, Math.min(natural, fit));
  const width = spread * Math.max(0, count - 1) + HAND_CARD_WIDTH;
  const center = Math.max(clearLeft + width / 2, Math.min(stageWidth / 2, stageWidth - clearRight - width / 2));
  return { center, spread, width };
}

/** The wide layout's columns: seats on the left, stack and decision corner on the right. */
const FIELD_LEFT = 300;
/** wide: your seat's margin from the stage's bottom-left corner */
const WIDE_SEAT_MARGIN = 24;
/** wide: the decision corner (ActionCluster: 420 wide, 40 from the edge) and a gap */
const WIDE_CORNER = 472;
const FIELD_RIGHT_MARGIN = 470;
/** multiplayer: each opponent's seat (plate, counters) in the left column, this far apart */
const SEAT_SPACING = 172;
/**
 * A wide stage taller than 1080 (a 16:10 or 4:3 window) splits its extra height between the two halves; each half
 * spends most of it on bigger cards and the rest on the gaps between the rows.
 */
const WIDE_EXTRA_FRONT = 0.2;
const WIDE_EXTRA_BACK = 0.15;

/** The tall layout: margins, the bands above and below the battlefield, and the rows between them. */
const TALL_MARGIN = 24;
const TALL_TOP_BAND = 215;
/** your seat, piles and decision corner, above the hand */
const TALL_BOTTOM_BAND = 180;
/** the hand's visible edge at the bottom of the stage */
const TALL_HAND = 130;
const TALL_ROW_GAP = 10;
const TALL_FRONT_IDEAL = 140;
const TALL_BACK_IDEAL = 104;
/** a taller window (a 16:10 tablet) shares its extra height between bigger rows and a higher hand */
const TALL_EXTRA_FRONT = 0.12;
const TALL_EXTRA_BACK = 0.09;
const TALL_EXTRA_HAND = 0.4;
const TALL_MAX_HAND_RAISE = 110;
const CARD_RATIO = 88 / 63;

const cache = new Map<string, MatchLayout>();

/**
 * The layout for a stage size. The same size gives the same objects (rows included), so memoized zones don't
 * re-render because the layout was worked out again.
 */
export function matchLayout(stage: Pick<StageContextValue, 'layout' | 'width' | 'height'>): MatchLayout {
  const key = `${stage.layout}:${stage.width}:${stage.height}`;
  let layout = cache.get(key);
  if (!layout) {
    if (cache.size > 16) cache.clear();
    layout = computeLayout(stage);
    const columns = new Map<string, RowsPlacement>();
    const them = layout.them;
    layout.them = (index, count) => {
      const column = `${index}/${count}`;
      let place = columns.get(column);
      if (!place) {
        place = them(index, count);
        columns.set(column, place);
      }
      return place;
    };
    cache.set(key, layout);
  }
  return layout;
}

function computeLayout(stage: Pick<StageContextValue, 'layout' | 'width' | 'height'>): MatchLayout {
  if (stage.layout === 'tall') return tallLayout(stage.width, stage.height);
  const fieldWidth = stage.width - FIELD_LEFT - FIELD_RIGHT_MARGIN;
  const extra = Math.max(0, stage.height - STAGE_HEIGHT);
  const half = Math.round(extra / 2);
  const frontIdeal = Math.round(150 + extra * WIDE_EXTRA_FRONT);
  const backIdeal = Math.round(112 + extra * WIDE_EXTRA_BACK);
  const frontGrowth = Math.round((frontIdeal - 150) * CARD_RATIO);
  const backGrowth = Math.round((backIdeal - 112) * CARD_RATIO);
  return {
    tall: false,
    fieldLeft: FIELD_LEFT,
    fieldWidth,
    me: { left: FIELD_LEFT, width: fieldWidth, frontTop: 556 + half, backTop: 778 + half + frontGrowth, frontIdeal, backIdeal },
    them: (index, count) => {
      const width = fieldWidth / count;
      return {
        left: FIELD_LEFT + index * width,
        width: width - (count > 1 ? 24 : 0),
        frontTop: (count === 1 ? 330 : 300) + backGrowth,
        backTop: count === 1 ? 160 : 150,
        frontIdeal,
        backIdeal,
      };
    },
    seam: STAGE_HEIGHT / 2 + half,
    playLine: stage.height - 300,
    myPlate: { left: WIDE_SEAT_MARGIN, bottom: WIDE_SEAT_MARGIN },
    myPiles: { left: 24, top: 600 + half },
    theirPlate: (index, count) => ({ left: 24, top: count === 1 ? 24 : 16 + index * SEAT_SPACING }),
    theirPiles: { left: 24, top: 140 },
    hiddenHandX: (index, count) => FIELD_LEFT + (index + 0.5) * (fieldWidth / count),
    stackPoint: { x: stage.width - 341, y: 470 + half },
    handRaise: 0,
    handClearRight: WIDE_CORNER,
  };
}

function tallLayout(width: number, height: number): MatchLayout {
  const fieldLeft = TALL_MARGIN;
  const fieldWidth = width - 2 * TALL_MARGIN;
  const extra = Math.max(0, height - MIN_TALL_HEIGHT);
  const frontIdeal = Math.round(TALL_FRONT_IDEAL + extra * TALL_EXTRA_FRONT);
  const backIdeal = Math.round(TALL_BACK_IDEAL + extra * TALL_EXTRA_BACK);
  const handRaise = Math.round(Math.min(TALL_MAX_HAND_RAISE, extra * TALL_EXTRA_HAND));
  const frontHeight = frontIdeal * CARD_RATIO;
  const backHeight = backIdeal * CARD_RATIO;
  const bandTop = height - TALL_HAND - handRaise - TALL_BOTTOM_BAND;
  const myBack = bandTop - TALL_ROW_GAP - backHeight;
  const myFront = myBack - TALL_ROW_GAP - frontHeight;
  const theirBack = TALL_TOP_BAND;
  const theirFront = theirBack + backHeight + TALL_ROW_GAP;
  const seam = Math.round((theirFront + frontHeight + myFront) / 2);
  const column = (index: number, count: number) => fieldWidth / count * index;
  return {
    tall: true,
    fieldLeft,
    fieldWidth,
    me: { left: fieldLeft, width: fieldWidth, frontTop: myFront, backTop: myBack, frontIdeal, backIdeal },
    them: (index, count) => ({
      left: fieldLeft + column(index, count),
      width: fieldWidth / count - (count > 1 ? 16 : 0),
      frontTop: theirFront,
      backTop: theirBack,
      frontIdeal,
      backIdeal,
    }),
    seam,
    playLine: bandTop,
    myPlate: { left: TALL_MARGIN, top: bandTop + 20 },
    myPiles: { left: 290, top: bandTop + 8 },
    theirPlate: (index, count) => ({ left: fieldLeft + column(index, count), top: count === 1 ? 24 : 16 }),
    theirPiles: { left: 290, top: 52 },
    hiddenHandX: (index, count) => (count === 1 ? width - 330 : fieldLeft + column(index + 1, count) - 120),
    stackPoint: { x: TALL_MARGIN + 95, y: seam - 180 },
    handRaise,
    handClearRight: 0,
  };
}
