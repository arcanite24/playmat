// @vitest-environment happy-dom
import { beforeEach, expect, test, vi } from 'vitest';

const loadGamepad = () => import('./gamepad');
beforeEach(() => {
  vi.resetModules();
  document.body.innerHTML = '';
});

const BUTTON = { A: 0, B: 1, RT: 7, UP: 12, DOWN: 13, LEFT: 14, RIGHT: 15 } as const;

function place(positions: Readonly<Record<string, readonly [number, number]>>) {
  Object.entries(positions).forEach(([id, [x, y]]) => {
    const element = document.getElementById(id)!;
    element.getClientRects = () => [{}] as unknown as DOMRectList;
    element.getBoundingClientRect = () => ({ left: x, top: y, right: x + 50, bottom: y + 50, width: 50, height: 50 }) as DOMRect;
  });
}

const focusedId = () => document.activeElement?.id;

test('A presses a focused button unless a handler took the Enter', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<button id="go">Go</button>';
  const onClick = vi.fn();
  const goButton = document.getElementById('go')!;
  goButton.addEventListener('click', onClick);
  goButton.focus();
  press(BUTTON_ACTIONS[BUTTON.A]);
  expect(onClick).toHaveBeenCalledTimes(1);
  expect(document.documentElement.dataset.input).toBe('gamepad');
  goButton.addEventListener('keydown', (event) => event.preventDefault());
  press(BUTTON_ACTIONS[BUTTON.A]);
  expect(onClick).toHaveBeenCalledTimes(1);
});

test('the D-pad moves to the nearest control that way, inside the top dialog only, and stops at the edge', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<button id="out">x</button><div role="dialog">'
    + '<button id="a">a</button><button id="b">b</button><button id="c">c</button><button id="d">d</button></div>';
  place({ out: [0, 0], a: [100, 100], b: [200, 100], c: [100, 200], d: [200, 200] });
  const pressAndRead = (button: number) => {
    press(BUTTON_ACTIONS[button]);
    return focusedId();
  };
  document.getElementById('a')!.focus();
  expect(pressAndRead(BUTTON.RIGHT)).toBe('b');
  expect(pressAndRead(BUTTON.DOWN)).toBe('d');
  expect(pressAndRead(BUTTON.LEFT)).toBe('c');
  expect(pressAndRead(BUTTON.UP)).toBe('a');
  expect(pressAndRead(BUTTON.UP)).toBe('a'); // nothing above inside the dialog: stays
});

test('RT opens the focused card as a right click does', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<div tabindex="0" id="card" data-object-id="x"></div>';
  const onContextMenu = vi.fn();
  const card = document.getElementById('card')!;
  card.addEventListener('contextmenu', onContextMenu);
  card.focus();
  press(BUTTON_ACTIONS[BUTTON.RT]);
  expect(onContextMenu).toHaveBeenCalledTimes(1);
});

test('in a match the D-pad reaches only the game, browses unplayable cards, and regains focus after a card leaves', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<button id="chat">chat</button>'
    + '<div id="p1" data-object-id="p1" tabindex="-1"></div><div id="h1" data-object-id="h1" tabindex="0"></div>'
    + '<div id="h2" data-object-id="h2" tabindex="-1"></div><div id="hidden" data-object-id="x" data-hidden tabindex="-1"></div>';
  place({ chat: [120, 0], p1: [100, 100], h1: [100, 200], h2: [200, 200], hidden: [150, 200] });
  document.getElementById('h1')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RIGHT]);
  expect(focusedId()).toBe('h2'); // past the face-down card, onto one that can't be played
  press(BUTTON_ACTIONS[BUTTON.UP]);
  expect(focusedId()).toBe('p1');
  press(BUTTON_ACTIONS[BUTTON.UP]);
  expect(focusedId()).toBe('p1'); // chat is above, but not on the D-pad
  document.getElementById('h2')!.focus();
  press(BUTTON_ACTIONS[BUTTON.A]);
  document.getElementById('h2')!.remove();
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('h1');
});

test('with a card detail open, the D-pad opens the next card instead', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<div id="table"><div id="a" data-object-id="a" tabindex="0"></div>'
    + '<div id="b" data-object-id="b" tabindex="-1"></div><div id="me" data-object-id="me" data-player tabindex="0"></div></div>';
  place({ a: [100, 100], b: [200, 100], me: [300, 100] });
  const onContextMenu = vi.fn();
  document.getElementById('table')!.addEventListener('contextmenu', onContextMenu);
  document.getElementById('a')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RT]);
  document.body.insertAdjacentHTML('beforeend', '<div id="sheet" role="dialog" data-card-detail tabindex="-1"><button>x</button></div>');
  document.getElementById('sheet')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RIGHT]);
  press(BUTTON_ACTIONS[BUTTON.RIGHT]); // nothing but a player further right: stays on b
  expect(onContextMenu.mock.calls.map(([event]) => (event.target as HTMLElement).id)).toEqual(['a', 'b']);
});

test('left and right stay in the hand, even with a lifted card nearer the board', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<div id="land" data-object-id="land" tabindex="-1"></div>'
    + '<div id="hand" role="list"><div id="c1" data-object-id="c1" tabindex="0"></div><div id="c2" data-object-id="c2" tabindex="0"></div>'
    + '<div id="c3" data-object-id="c3" tabindex="0"></div></div>';
  place({ land: [150, 0], c1: [100, 300], c2: [150, 300], c3: [200, 120] });
  document.getElementById('c3')!.focus();
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('c2');
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('c1'); // the last card on the left is reachable
});

test('on a wrapped battlefield row, left and right keep to the line; up and down change it', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<div data-label="Creatures">'
    + ['a', 'b', 'c', 'd'].map((id) => `<div id="${id}" data-object-id="${id}" tabindex="-1"></div>`).join('') + '</div>';
  place({ a: [100, 100], b: [200, 100], c: [150, 140], d: [250, 140] });
  document.getElementById('b')!.focus();
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('a'); // not c, though c is nearer
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('a'); // end of the line: stays
  press(BUTTON_ACTIONS[BUTTON.DOWN]);
  expect(focusedId()).toBe('c');
  press(BUTTON_ACTIONS[BUTTON.RIGHT]);
  expect(focusedId()).toBe('d');
});

test('past the end of a line, right reaches the turn buttons and left the piles and mana', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<div data-label="Lands"><div id="land" data-object-id="land" tabindex="-1"></div></div>'
    + '<div role="list"><div id="c1" data-object-id="c1" tabindex="0"></div></div>'
    + '<section data-pad data-turn><button id="resolve">Resolve</button></section>'
    + '<div data-pad><button id="pool">G</button></div>'
    + '<div data-label="Creatures"><div id="mine" data-object-id="mine" tabindex="-1"></div></div>';
  place({ land: [400, 200], c1: [100, 400], resolve: [500, 330], pool: [0, 380], mine: [100, 300] });
  document.getElementById('c1')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RIGHT]);
  expect(focusedId()).toBe('resolve');
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('c1');
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('pool');
  document.getElementById('mine')!.focus();
  press(BUTTON_ACTIONS[BUTTON.LEFT]);
  expect(focusedId()).toBe('pool');
});

test('a press fires once; a held direction repeats after a pause, a held face button does not', async () => {
  const { BUTTON_ACTIONS, holdStep } = await loadGamepad();
  const down = BUTTON_ACTIONS[BUTTON.DOWN];
  expect(holdStep(undefined, down, 0)).toEqual({ fires: true, due: 400 });
  expect(holdStep(400, down, 300)).toEqual({ fires: false, due: 400 });
  expect(holdStep(400, down, 450)).toEqual({ fires: true, due: 570 });
  expect(holdStep(400, BUTTON_ACTIONS[BUTTON.B], 1000)).toEqual({ fires: false, due: 400 });
});

test('a released button fires again on the next press', async () => {
  const { BUTTON_ACTIONS, readButton } = await loadGamepad();
  const onKeyDown = vi.fn();
  document.body.addEventListener('keydown', onKeyDown);
  readButton('pad:B', BUTTON_ACTIONS[BUTTON.B], true, 0);
  readButton('pad:B', BUTTON_ACTIONS[BUTTON.B], true, 1000);
  readButton('pad:B', BUTTON_ACTIONS[BUTTON.B], false, 1100);
  readButton('pad:B', BUTTON_ACTIONS[BUTTON.B], true, 1200);
  document.body.removeEventListener('keydown', onKeyDown);
  expect(onKeyDown).toHaveBeenCalledTimes(2);
});

test('with a card detail open from a zone viewer, the D-pad flips within that viewer', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<div id="board"><div id="b1" data-object-id="b1" tabindex="0"></div></div>'
    + '<div id="viewer" role="dialog"><div id="g1" data-card-id="g1" tabindex="0"></div><div id="g2" data-card-id="g2" tabindex="0"></div></div>';
  place({ b1: [150, 100], g1: [100, 100], g2: [200, 100] });
  const onContextMenu = vi.fn();
  document.body.addEventListener('contextmenu', onContextMenu);
  document.getElementById('b1')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RIGHT]); // remembers board focus
  document.getElementById('g1')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RT]);
  document.body.insertAdjacentHTML('beforeend', '<div id="sheet" role="dialog" data-card-detail tabindex="-1"></div>');
  document.getElementById('sheet')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RIGHT]);
  document.body.removeEventListener('contextmenu', onContextMenu);
  expect(onContextMenu.mock.calls.map(([event]) => (event.target as HTMLElement).id)).toEqual(['g1', 'g2']);
});

test('a non-match role=list grid keeps horizontal moves on the same row', async () => {
  const { BUTTON_ACTIONS, press } = await loadGamepad();
  document.body.innerHTML = '<div role="list"><button id="r1c1">1</button><button id="r2c2">2</button></div>';
  place({ r1c1: [100, 100], r2c2: [200, 220] });
  document.getElementById('r1c1')!.focus();
  press(BUTTON_ACTIONS[BUTTON.RIGHT]);
  expect(focusedId()).toBe('r1c1');
});
