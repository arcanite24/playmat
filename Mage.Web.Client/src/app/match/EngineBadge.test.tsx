import userEvent from '@testing-library/user-event';
import { render, screen } from '@testing-library/react';
import { beforeAll, beforeEach, describe, expect, test } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { registerMessages } from '../i18n';
import match from '../i18n/en/match';
import { EngineBadge } from './EngineBadge';
import { engineOf } from './engine';

const rust = { engine: 'rust' } as GameView;
const java = {} as GameView;

describe('EngineBadge', () => {
  beforeAll(() => registerMessages(match));
  beforeEach(() => localStorage.clear());

  test('the engine is Rust only when the view says so', () => {
    expect(engineOf(rust)).toBe('rust');
    expect(engineOf(java)).toBe('java');
    expect(engineOf(null)).toBe('java');
  });

  test('a Java game shows nothing', () => {
    const { container } = render(<EngineBadge gameId="g-java" view={java} showNotice />);
    expect(container.innerHTML).toBe('');
  });

  test('a Rust game shows the chip, and the note once', async () => {
    const { rerender } = render(<EngineBadge gameId="g-rust" view={rust} showNotice />);
    expect(screen.getByRole('status').textContent).toMatch(/rust engine/i);
    expect(screen.getByRole('dialog').textContent).toMatch(/playing on the rust engine/i);
    await userEvent.click(screen.getByRole('button', { name: /got it/i }));
    expect(screen.queryByRole('dialog')).toBeNull();
    // the same game again (a reconnect): no note, the chip stays
    rerender(<EngineBadge gameId="g-rust" view={{ ...rust }} showNotice />);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByRole('status')).toBeTruthy();
  });

  test("don't show again holds for later games", async () => {
    render(<EngineBadge gameId="g-1" view={rust} showNotice />);
    await userEvent.click(screen.getByRole('checkbox'));
    await userEvent.click(screen.getByRole('button', { name: /got it/i }));
    render(<EngineBadge gameId="g-2" view={rust} showNotice />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  test('no note when watching or after the game', () => {
    render(<EngineBadge gameId="g-watch" view={rust} showNotice={false} />);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByRole('status')).toBeTruthy();
  });
});
