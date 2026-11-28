import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { registerMessages } from '../i18n';
import home from '../i18n/en/home';
import Customize from './Customize';
import { defaultLayout, preset } from './layoutModel';
import { useHomeLayout } from './layoutStore';

const homeLayoutSet = vi.fn(async (layout: string) => layout);

vi.mock(import('../connection'), async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    api: { ...actual.api, homeLayoutGet: async () => null, homeLayoutSet: (layout: string) => homeLayoutSet(layout) },
  };
});

// the widgets themselves fetch; here only their frames matter
vi.mock('./WidgetFrame', () => ({
  WidgetFrame: ({ item }: { item: { id: string } }) => <p>{item.id}</p>,
}));

registerMessages(home);

const has = { career: true, ranked: true };

function order(): string[] {
  return [...document.querySelectorAll<HTMLElement>('[data-key]')].map((cell) => cell.dataset.key ?? '');
}

describe('Customize', () => {
  beforeEach(() => {
    homeLayoutSet.mockClear();
    localStorage.clear();
    useHomeLayout.setState({ user: 'tester', layout: defaultLayout(), synced: true, error: null });
  });

  test('a preset and Done save the layout to the account', async () => {
    const onDone = vi.fn();
    render(<Customize has={has} onDone={onDone} />);
    fireEvent.click(screen.getByRole('button', { name: 'Competitor' }));
    expect(screen.getByRole('button', { name: 'Competitor' }).getAttribute('aria-pressed')).toBe('true');
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Done' }));
    });
    expect(onDone).toHaveBeenCalled();
    expect(homeLayoutSet).toHaveBeenCalledTimes(1);
    expect(JSON.parse(homeLayoutSet.mock.calls[0][0])).toEqual(preset('competitor'));
    expect(useHomeLayout.getState().layout.preset).toBe('competitor');
    expect(localStorage.getItem('playmat.homeLayout.tester')).not.toBeNull();
  });

  test('Escape leaves without saving', () => {
    const onDone = vi.fn();
    render(<Customize has={has} onDone={onDone} />);
    fireEvent.click(screen.getByRole('button', { name: 'Minimal' }));
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onDone).toHaveBeenCalled();
    expect(homeLayoutSet).not.toHaveBeenCalled();
    expect(useHomeLayout.getState().layout.preset).toBe('casual');
  });

  test('the arrow keys move a widget and say where it went', () => {
    render(<Customize has={has} onDone={() => undefined} />);
    const first = order()[0];
    const handle = screen.getByRole('button', { name: 'Move Quests' });
    fireEvent.keyDown(handle, { key: 'ArrowRight' });
    expect(order()[1]).toBe(first);
    expect(screen.getByText(/Quests moved to place 2 of/)).toBeTruthy();
    expect(document.activeElement?.getAttribute('aria-label')).toBe('Move Quests');
  });

  test('a removed widget comes back with Undo, in its place', () => {
    render(<Customize has={has} onDone={() => undefined} />);
    const before = order();
    fireEvent.click(screen.getByRole('button', { name: 'Remove Daily wins' }));
    expect(order()).not.toContain(before[1]);
    fireEvent.click(screen.getByRole('button', { name: 'Undo' }));
    expect(order()).toEqual(before);
  });

  test('widgets this server cannot show are left out of the gallery', () => {
    render(<Customize has={{ career: false, ranked: false }} onDone={() => undefined} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add widget' }));
    const dialog = screen.getByRole('dialog', { name: 'Add a widget' });
    expect(dialog.textContent).toContain('Saved games');
    expect(dialog.textContent).not.toContain('Quests');
  });
});
