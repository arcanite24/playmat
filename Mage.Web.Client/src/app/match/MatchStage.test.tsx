import { QueryClientProvider } from '@tanstack/react-query';
import { act, render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { GameSession } from '../../core/game/gameSession';
import { EventBus } from '../../core/rpc/EventBus';
import { loadTestCatalog } from '../../test/catalog';
import { api, rpc } from '../connection';
import { queryClient } from '../queries';
import { useDecks } from '../stores/decks';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { rememberLook } from '../support/supportData';
import { MatchStage } from './MatchStage';
import type { MatCloth } from './playmats';

vi.mock('./MatPrint', () => ({
  MatPrint: ({ cloth, enabled }: { cloth: MatCloth | null; enabled: boolean }) => <div data-cloth={cloth?.id} data-print={enabled} />,
}));

beforeEach(() => {
  vi.useFakeTimers();
  loadTestCatalog();
  queryClient.setQueryData(['serverInfo'], { accounts: true });
  useSession.setState({ phase: 'signedIn', userName: 'Alice' });
  usePlay.setState({ deckId: 'starter:test', returnPath: null });
  useDecks.setState({
    saved: [], sleeves: { 'starter:test': '#6e3f24' },
    starters: [{ file: 'test', name: 'Test', cards: 60, cover: { name: 'Forest', setCode: 'M21', cardNumber: '274' } }],
  });
  useSettings.getState().update({ matCloth: 'sand', matArt: 'deck', sound: false });
  rememberLook({ name: 'Alice', sleeve: '#20302b', cloth: 'navy' });
  rememberLook({ name: 'Bob', sleeve: '#1f2a44', cloth: 'plum' });
});

afterEach(() => {
  queryClient.clear();
  vi.clearAllTimers();
  vi.useRealTimers();
});

function match(watch = false) {
  const bus = new EventBus(rpc);
  const session = new GameSession(api, bus, { gameId: 'g1', playerId: watch ? null : 'me', mode: watch ? 'watch' : 'play' });
  const state = {
    ...session.getState(),
    view: { players: [
      { playerId: 'me', name: 'Alice', life: 20, handCount: 1 },
      { playerId: 'them', name: 'Bob', life: 20, handCount: 1 },
    ] },
  };
  const rendered = render(<QueryClientProvider client={queryClient}><MemoryRouter><MatchStage session={session} state={state} /></MemoryRouter></QueryClientProvider>);
  return { ...rendered, dispose: () => { rendered.unmount(); session.dispose(); bus.dispose(); } };
}

test('the own seat uses the local deck sleeve and cloth immediately, even while the server look is stale', () => {
  const { container, dispose } = match();
  const ownPlate = container.querySelector('[data-object-id="me"]');
  expect(ownPlate).toBeTruthy();
  expect(container.querySelector('[data-cloth="sand"]')).toBeTruthy();
  expect(container.querySelector('[data-cloth="plum"]')).toBeTruthy();
  expect(container.querySelector('[data-cloth="navy"]')).toBeNull();
  expect(ownPlate?.getAttribute('style')).toContain('#6e3f24');
  act(() => useSettings.getState().update({ matCloth: 'teal' }));
  expect(container.querySelector('[data-cloth="teal"]')).toBeTruthy();
  act(() => useDecks.setState({ sleeves: { 'starter:test': '#24403f' } }));
  expect(ownPlate?.getAttribute('style')).toContain('#24403f');
  act(() => rememberLook({ name: 'Alice', sleeve: '#5a2320', cloth: 'oxblood' }));
  expect(ownPlate?.getAttribute('style')).toContain('#24403f');
  expect(container.querySelector('[data-cloth="teal"]')).toBeTruthy();
  act(() => useSettings.getState().update({ matArt: 'none' }));
  expect(container.querySelectorAll('[data-print="false"]').length).toBe(2);
  dispose();
});

test('watching uses the seated players server looks and contrasts identical sleeves', () => {
  rememberLook({ name: 'Alice', sleeve: '#1f2a44', cloth: 'navy' });
  const { container, dispose } = match(true);
  expect(container.querySelector('[data-cloth="navy"]')).toBeTruthy();
  expect(container.querySelector('[data-cloth="sand"]')).toBeNull();
  const ownPlate = container.querySelector('[data-object-id="me"]');
  const theirPlate = container.querySelector('[data-object-id="them"]');
  expect(ownPlate?.getAttribute('style')).toContain('#1f2a44');
  expect(theirPlate?.getAttribute('style')).not.toContain('#1f2a44');
  dispose();
});
