import { act, fireEvent, render, screen } from '@testing-library/react';
import { useSyncExternalStore } from 'react';
import { describe, expect, test, vi } from 'vitest';
import { EventBus } from '../../core/rpc/EventBus';
import type { ServerEvent } from '../../core/rpc/RpcClient';
import { GameSession } from '../../core/game/gameSession';
import type { Api } from '../../protocol/generated/api';
import type { CardView, GameView } from '../../protocol/generated/views';
import { Hand } from './Hand';
import { ActionCluster } from './ActionCluster';
import { placeHand } from './matchLayout';
import { useMatchUi } from './matchUi';
import { DEFAULT_STAGE, StageContext } from './stageContext';

const BOLT: CardView = { id: 'bolt', name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' } as CardView;
const BEAR: CardView = { id: 'bear', name: 'Grizzly Bears', expansionSetCode: 'M10', cardNumber: '168' } as CardView;
const PLAY_LINE = 600;

/** A real GameSession on a real EventBus, fed by hand; the api records what the client sends. */
function setup() {
  const listeners = new Set<(event: ServerEvent) => void>();
  const bus = new EventBus({
    onEvent: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    onSessionStart: () => () => undefined,
    onStatus: () => () => undefined,
  });
  const api = {
    sendPlayerUUID: vi.fn(async () => undefined),
    sendPlayerBoolean: vi.fn(async () => undefined),
  };
  const session = new GameSession(api as unknown as Api, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
  let messageId = 0;
  const emit = (method: ServerEvent['method'], data: unknown) =>
    act(() => listeners.forEach((listener) => listener({ method, data, objectId: 'g1', messageId: ++messageId } as ServerEvent)));

  const view: GameView = {
    myPlayerId: 'me',
    activePlayerId: 'me',
    priorityPlayerName: 'me',
    turn: 2,
    step: 'PRECOMBAT_MAIN',
    canPlayObjects: { objects: { bolt: { basicPlayAbilities: [{}] } } },
  } as unknown as GameView;

  function Harness() {
    const state = useSyncExternalStore(session.store.subscribe, session.store.getState);
    const clickable = new Set(state.interaction.clickable.keys());
    return (
      <StageContext.Provider value={DEFAULT_STAGE}>
        <Hand
          cards={[BOLT, BEAR]}
          clickable={clickable}
          selected={state.interaction.selected}
          sleeve="default"
          onPlay={(id) => session.click(id)}
          playLine={PLAY_LINE}
          libraryOrigin="library:me"
        />
      </StageContext.Provider>
    );
  }

  render(<Harness />);
  emit('GAME_SELECT', { message: 'Play spells and abilities.', gameView: view });
  return { api, session, emit };
}

const card = (name: RegExp) => screen.getByRole('listitem', { name });

describe('Hand', () => {
  test('labels the hand and marks playable cards', () => {
    setup();
    expect(screen.getByRole('list', { name: 'Your hand, 2 cards' })).toBeTruthy();
    expect(card(/Lightning Bolt, playable/).tabIndex).toBe(0);
    expect(card(/^Grizzly Bears$/).tabIndex).toBe(-1);
  });

  test('clicking a playable card casts it', () => {
    const { api } = setup();
    const bolt = card(/Lightning Bolt/);
    fireEvent.pointerDown(bolt, { button: 0, pointerId: 1, clientX: 100, clientY: 900 });
    fireEvent.pointerUp(bolt, { button: 0, pointerId: 1, clientX: 102, clientY: 901 });
    expect(api.sendPlayerUUID).toHaveBeenCalledWith('g1', 'bolt');
  });

  test('dragging a card above the play line casts it; dropping it back in hand does not', () => {
    const { api, emit } = setup();
    let bolt = card(/Lightning Bolt/);
    fireEvent.pointerDown(bolt, { button: 0, pointerId: 1, clientX: 100, clientY: 900 });
    fireEvent.pointerMove(bolt, { pointerId: 1, clientX: 110, clientY: 860 });
    fireEvent.pointerUp(bolt, { pointerId: 1, clientX: 110, clientY: 850 });
    expect(api.sendPlayerUUID).not.toHaveBeenCalled();

    // a fresh prompt (the click above was not sent, but be explicit about state)
    emit('GAME_SELECT', { message: 'Play spells and abilities.', gameView: { myPlayerId: 'me', activePlayerId: 'me', turn: 2, canPlayObjects: { objects: { bolt: {} } } } });
    bolt = card(/Lightning Bolt/);
    fireEvent.pointerDown(bolt, { button: 0, pointerId: 2, clientX: 100, clientY: 900 });
    fireEvent.pointerMove(bolt, { pointerId: 2, clientX: 120, clientY: 500 });
    fireEvent.pointerUp(bolt, { pointerId: 2, clientX: 120, clientY: 400 });
    expect(api.sendPlayerUUID).toHaveBeenCalledTimes(1);
    expect(api.sendPlayerUUID).toHaveBeenCalledWith('g1', 'bolt');
  });

  test('cards that are not playable ignore clicks and drags', () => {
    const { api } = setup();
    const bear = card(/Grizzly Bears/);
    fireEvent.pointerDown(bear, { button: 0, pointerId: 1, clientX: 100, clientY: 900 });
    fireEvent.pointerMove(bear, { pointerId: 1, clientX: 100, clientY: 300 });
    fireEvent.pointerUp(bear, { pointerId: 1, clientX: 100, clientY: 300 });
    fireEvent.keyDown(bear, { key: 'Enter' });
    expect(api.sendPlayerUUID).not.toHaveBeenCalled();
  });

  test('Enter on a focused playable card casts it, and a second click waits for the server', () => {
    const { api } = setup();
    const bolt = card(/Lightning Bolt/);
    fireEvent.keyDown(bolt, { key: 'Enter' });
    fireEvent.keyDown(bolt, { key: 'Enter' });
    expect(api.sendPlayerUUID).toHaveBeenCalledTimes(1);
    expect(api.sendPlayerUUID).toHaveBeenCalledWith('g1', 'bolt');
  });

  test('a right click does not cast', () => {
    const { api } = setup();
    const bolt = card(/Lightning Bolt/);
    fireEvent.pointerDown(bolt, { button: 2, pointerId: 1, clientX: 100, clientY: 900 });
    fireEvent.pointerUp(bolt, { button: 2, pointerId: 1, clientX: 100, clientY: 900 });
    expect(api.sendPlayerUUID).not.toHaveBeenCalled();
  });

  test('in a big hand a mouse lifts the card its x falls on, one card per slice, not the wide lifted card it rests on', () => {
    const cards = Array.from({ length: 20 }, (_, index) => ({ id: `c${index}`, name: `Card ${index}` }) as CardView);
    const { spread } = placeHand(cards.length, DEFAULT_STAGE.width);
    render(
      <StageContext.Provider value={DEFAULT_STAGE}>
        <Hand cards={cards} clickable={new Set()} selected={new Set()} sleeve="default" onPlay={() => undefined} playLine={PLAY_LINE} libraryOrigin="library:me" />
      </StageContext.Provider>,
    );
    const hand = screen.getByRole('list', { name: 'Your hand, 20 cards' });
    const lifted = () => screen.getAllByRole('listitem').filter((item) => item.className.includes('lifted')).map((item) => item.getAttribute('aria-label'));
    fireEvent.pointerMove(hand, { pointerType: 'mouse', clientX: spread * 5 + 2, clientY: 900 });
    expect(lifted()).toEqual(['Card 5']);
    expect(useMatchUi.getState().zoom?.card.name).toBe('Card 5');
    fireEvent.pointerMove(hand, { pointerType: 'mouse', clientX: spread * 6 + 2, clientY: 900 });
    expect(lifted()).toEqual(['Card 6']);
    fireEvent.pointerLeave(hand, { pointerType: 'mouse' });
    expect(lifted()).toEqual([]);
    expect(useMatchUi.getState().zoom).toBeNull();
  });
});

describe('hand number shortcuts', () => {
  test('plays the visible numbered card only when eligible, once until the server answers', () => {
    const { api } = setup();
    fireEvent.keyDown(window, { key: '2' });
    fireEvent.keyDown(window, { key: '1', altKey: true });
    fireEvent.keyDown(window, { key: '1', ctrlKey: true });
    fireEvent.keyDown(window, { key: '1', repeat: true });
    expect(api.sendPlayerUUID).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: '1' });
    expect(api.sendPlayerUUID).toHaveBeenCalledTimes(1);
    expect(api.sendPlayerUUID).toHaveBeenCalledWith('g1', 'bolt');
  });

  test('activating a focused hand card never also passes priority', () => {
    const { api } = setup();
    const onCommand = vi.fn();
    render(<ActionCluster interaction={{ mode: 'priority', prompt: null, headline: '', clickable: new Map(), selected: new Set(), mainButton: { label: 'Pass', tone: 'primary', command: { type: 'boolean', value: false } }, secondaryButtons: [] }} awaiting={false} status={null} canAct special={false} holdingPriority={false} onCommand={onCommand} />);
    fireEvent.keyDown(card(/Lightning Bolt/), { key: 'Enter' });
    expect(api.sendPlayerUUID).toHaveBeenCalledTimes(1);
    expect(onCommand).not.toHaveBeenCalled();
  });

  test('does not play cards behind dialogs or while typing', () => {
    const { api } = setup();
    const overlay = render(<div role="dialog"><textarea aria-label="Chat" /></div>);
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(screen.getByRole('textbox'), { key: '1' });
    overlay.unmount();
    expect(api.sendPlayerUUID).not.toHaveBeenCalled();
  });
});
