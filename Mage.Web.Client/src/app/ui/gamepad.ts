/** Controller support: buttons send the keys the app already handles. Pure decisions first, DOM shell last. */
import { useShellUi } from '../stores/shellUi';

type Direction = 'up' | 'down' | 'left' | 'right';
type TabDirection = 'next' | 'previous';
type ButtonAction = Readonly<{
  key?: string;
  altKey?: boolean;
  shiftKey?: boolean;
  nav?: TabDirection | Direction;
  menu?: true;
  settings?: true;
}>;

// standard layout indices
export const BUTTON_ACTIONS: Readonly<Record<number, ButtonAction>> = {
  0: { key: 'Enter' }, // A
  1: { key: 'Escape' }, // B
  2: { key: 'Enter', altKey: true }, // X
  3: { key: 'Enter', shiftKey: true }, // Y
  4: { key: 'Tab', shiftKey: true, nav: 'previous' }, // LB
  5: { key: 'Tab', nav: 'next' }, // RB
  7: { menu: true }, // RT: card detail
  9: { settings: true }, // Start
  12: { key: 'ArrowUp', nav: 'up' },
  13: { key: 'ArrowDown', nav: 'down' },
  14: { key: 'ArrowLeft', nav: 'left' },
  15: { key: 'ArrowRight', nav: 'right' },
};

const REPEAT_DELAY_MS = 400;
const REPEAT_EVERY_MS = 120;
const ACROSS_WEIGHT = 3;

const DIALOGS = '[role="dialog"], [role="alertdialog"], [role="menu"]';
const TABBABLE = 'a[href], button:not(:disabled), input:not(:disabled):not([type="hidden"]), select:not(:disabled), '
  + 'textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';
// in a match the D-pad only reaches the game: cards, players and [data-pad] regions, not chat or the menu
const DPAD_TARGETS = ['[data-object-id][tabindex]:not([data-hidden])', ...TABBABLE.split(', ').map((selector) => `[data-pad] ${selector}`)].join(', ');
const DETAIL_CARDS = '[data-object-id]:not([data-hidden]):not([data-player])';

// ---------- pure core ----------

type Target = Readonly<{
  element: HTMLElement;
  rect: DOMRect;
  handList: Element | null;
  isTurnButton: boolean;
  isOnBattlefield: boolean;
}>;

const centreOf = (rect: DOMRect) => ({ x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 });
const distanceBetween = (from: DOMRect, to: DOMRect) => Math.hypot(centreOf(from).x - centreOf(to).x, centreOf(from).y - centreOf(to).y);
const spanGap = (aStart: number, aEnd: number, bStart: number, bEnd: number) => Math.max(0, bStart - aEnd, aStart - bEnd);
const isHorizontal = (direction: Direction) => direction === 'left' || direction === 'right';

// sideways distance is the gap between boxes, so a lifted hand card still counts as level with its neighbours
function nearestTowards(from: DOMRect, candidates: readonly Target[], direction: Direction): Target | undefined {
  const horizontal = isHorizontal(direction);
  const sign = direction === 'left' || direction === 'up' ? -1 : 1;
  const axisOf = (rect: DOMRect) => (horizontal ? centreOf(rect).x : centreOf(rect).y);
  const acrossOf = (rect: DOMRect) => (horizontal ? spanGap(from.top, from.bottom, rect.top, rect.bottom) : spanGap(from.left, from.right, rect.left, rect.right));
  return candidates
    .map((candidate) => ({ candidate, along: sign * (axisOf(candidate.rect) - axisOf(from)) }))
    .filter(({ along }) => along > 1)
    .map(({ candidate, along }) => ({ candidate, score: along + acrossOf(candidate.rect) * ACROSS_WEIGHT }))
    .reduce<{ candidate: Target; score: number } | undefined>((best, scored) => (!best || scored.score < best.score ? scored : best), undefined)
    ?.candidate;
}

const isLevelWith = (rect: DOMRect) => (candidate: Target) => centreOf(candidate.rect).y >= rect.top && centreOf(candidate.rect).y <= rect.bottom;

// left/right stay on the line; past its end, right goes to the turn buttons and left off the battlefield
function neighbourTowards(from: Target, candidates: readonly Target[], direction: Direction): Target | undefined {
  const others = candidates.filter((candidate) => candidate.element !== from.element);
  if (!isHorizontal(direction)) return nearestTowards(from.rect, others, direction);
  const sameLine = others.filter((candidate) => (from.handList ? candidate.handList === from.handList : isLevelWith(from.rect)(candidate)));
  const beyondLine = others.filter((candidate) => (direction === 'right' ? candidate.isTurnButton : !candidate.isOnBattlefield));
  return nearestTowards(from.rect, sameLine, direction) ?? nearestTowards(from.rect, beyondLine, direction);
}

const closestTo = (point: DOMRect, candidates: readonly Target[]) =>
  candidates.reduce<Target | undefined>((best, candidate) => (!best || distanceBetween(point, candidate.rect) < distanceBetween(point, best.rect) ? candidate : best), undefined);

function inTabOrder(candidates: readonly Target[], currentIndex: number, direction: TabDirection): Target | undefined {
  const forwards = direction === 'next';
  if (currentIndex < 0) return forwards ? candidates[0] : candidates.at(-1);
  return candidates[(currentIndex + (forwards ? 1 : -1) + candidates.length) % candidates.length];
}

export function holdStep(due: number | undefined, action: ButtonAction, now: number): Readonly<{ fires: boolean; due: number }> {
  if (due === undefined) return { fires: true, due: now + REPEAT_DELAY_MS };
  if (action.nav && now >= due) return { fires: true, due: now + REPEAT_EVERY_MS };
  return { fires: false, due };
}

const isTabDirection = (nav: TabDirection | Direction): nav is TabDirection => nav === 'next' || nav === 'previous';
const isPlainEnter = (action: ButtonAction) => action.key === 'Enter' && !action.altKey && !action.shiftKey;

// ---------- shell ----------

type PadState = Readonly<{
  lastFocus?: Target;
  focusAfterDetail?: HTMLElement;
  holds: ReadonlyMap<string, number>;
  polling: boolean;
}>;

// the only mutable binding, always replaced whole
let padState: PadState = { holds: new Map(), polling: false };

const measure = (element: HTMLElement): Target => ({
  element,
  rect: element.getBoundingClientRect(),
  handList: element.closest('[role="list"]'),
  isTurnButton: !!element.closest('[data-turn]'),
  isOnBattlefield: !!element.closest('[data-label]'),
});

const isVisible = (element: Element) => element.getClientRects().length > 0;
const targetsIn = (root: ParentNode, selector: string) => Array.from(root.querySelectorAll<HTMLElement>(selector)).filter(isVisible).map(measure);
const topDialog = () => Array.from(document.querySelectorAll(DIALOGS)).at(-1);
const tabTargets = () => targetsIn(topDialog() ?? document, TABBABLE);
function dpadTargets() {
  const dialog = topDialog();
  if (dialog) return targetsIn(dialog, TABBABLE);
  const inMatch = !!document.querySelector('[data-object-id]');
  return targetsIn(document, inMatch ? DPAD_TARGETS : TABBABLE);
}
const bottomCentre = () => new DOMRect(window.innerWidth / 2, window.innerHeight, 0, 0);
const isActivatable = (target: Element): target is HTMLElement =>
  target instanceof HTMLElement && target.matches('button, a[href], summary, input[type="checkbox"], input[type="radio"]');

function rememberFocus(target: Target) {
  if (target.element.matches(DPAD_TARGETS)) padState = { ...padState, lastFocus: target };
}

function moveFocusTo(target: Target | undefined) {
  if (!target) return;
  target.element.focus();
  target.element.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  rememberFocus(target);
}

// CardDetail opens and closes on a right click
function rightClick(element: Element) {
  const { x, y } = centreOf(element.getBoundingClientRect());
  element.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: x, clientY: y }));
}

function flipDetail(direction: Direction) {
  const from = padState.lastFocus?.element;
  if (!from?.isConnected) return;
  const next = neighbourTowards(measure(from), targetsIn(document, DETAIL_CARDS), direction);
  if (!next) return;
  rightClick(next.element);
  // Radix returns focus to the first card on close, so remember where we ended up
  padState = { ...padState, lastFocus: next, focusAfterDetail: next.element };
}

// synthetic keys don't move focus, so do it ourselves
function navigate(nav: TabDirection | Direction) {
  if (!isTabDirection(nav) && topDialog()?.matches('[data-card-detail]')) {
    flipDetail(nav);
    return;
  }
  const candidates = isTabDirection(nav) ? tabTargets() : dpadTargets();
  const currentIndex = candidates.findIndex((candidate) => candidate.element === document.activeElement);
  if (isTabDirection(nav)) moveFocusTo(inTabOrder(candidates, currentIndex, nav));
  else if (currentIndex < 0) moveFocusTo(closestTo(padState.lastFocus?.rect ?? bottomCentre(), candidates));
  else moveFocusTo(neighbourTowards(candidates[currentIndex], candidates, nav));
}

// synthetic keys don't click either
function sendKey(target: Element, { nav, ...key }: ButtonAction) {
  const keydown = new KeyboardEvent('keydown', { ...key, bubbles: true, cancelable: true, composed: true });
  target.dispatchEvent(keydown);
  // clears held-key UI such as the Alt hints
  target.dispatchEvent(new KeyboardEvent('keyup', { key: key.key, bubbles: true, cancelable: true }));
  if (keydown.defaultPrevented) return;
  if (nav) navigate(nav);
  else if (isPlainEnter(key) && isActivatable(target)) target.click();
}

function toggleSettings() {
  const { settingsOpen, setSettingsOpen } = useShellUi.getState();
  setSettingsOpen(!settingsOpen);
}

export function press(action: ButtonAction) {
  document.documentElement.dataset.input = 'gamepad'; // large focus ring in base.css
  if (action.settings) {
    toggleSettings();
    return;
  }
  const target = document.activeElement ?? document.body;
  if (target instanceof HTMLElement) rememberFocus(measure(target));
  if (action.menu) rightClick(target);
  else sendKey(target, action);
}

export function readButton(holdKey: string, action: ButtonAction, pressed: boolean, now: number) {
  const due = padState.holds.get(holdKey);
  if (!pressed) {
    if (due !== undefined) padState = { ...padState, holds: new Map([...padState.holds].filter(([key]) => key !== holdKey)) };
    return;
  }
  const step = holdStep(due, action, now);
  if (step.due !== due) padState = { ...padState, holds: new Map([...padState.holds, [holdKey, step.due]]) };
  if (step.fires) press(action);
}

const shouldRegainFocus = () =>
  !!padState.lastFocus && document.documentElement.dataset.input === 'gamepad' && document.activeElement === document.body;

// a played card takes focus with it: bring focus back near where it was
function keepFocus() {
  const afterDetail = padState.focusAfterDetail;
  if (afterDetail && !document.querySelector('[data-card-detail]')) {
    padState = { ...padState, focusAfterDetail: undefined };
    if (afterDetail.isConnected) moveFocusTo(measure(afterDetail));
  } else if (shouldRegainFocus() && padState.lastFocus) {
    moveFocusTo(closestTo(padState.lastFocus.rect, dpadTargets()));
  }
}

function pollPads(now: number) {
  keepFocus();
  // pads with mapping "" (the AYN Thor) are read as standard
  const pads = navigator.getGamepads().filter((pad): pad is Gamepad => !!pad);
  pads
    .flatMap((pad) => Object.entries(BUTTON_ACTIONS).map(([index, action]) => ({ holdKey: `${pad.index}:${index}`, action, pressed: !!pad.buttons[Number(index)]?.pressed })))
    .forEach(({ holdKey, action, pressed }) => readButton(holdKey, action, pressed, now));
  padState = { ...padState, polling: pads.length > 0 };
  if (padState.polling) requestAnimationFrame(pollPads);
}

export function startGamepad() {
  if (typeof navigator === 'undefined' || !navigator.getGamepads) return;
  const leaveControllerMode = () => { delete document.documentElement.dataset.input; };
  window.addEventListener('pointerdown', leaveControllerMode, true);
  window.addEventListener('keydown', (event) => { if (event.isTrusted) leaveControllerMode(); }, true);
  window.addEventListener('gamepadconnected', () => {
    if (padState.polling) return;
    padState = { ...padState, polling: true };
    requestAnimationFrame(pollPads);
  });
}
