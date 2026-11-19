import { render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { registerMessages } from '../i18n';
import home from '../i18n/en/home';
import { WidgetFrame } from './WidgetFrame';

// one widget that works and one that throws, in place of the real registry
vi.mock('./widgets', () => {
  const defs = [
    { id: 'fine', sizes: ['s'], title: 'home.widget.level', blurb: 'home.widget.level.blurb', load: async () => ({ default: () => <p>all good</p> }) },
    {
      id: 'broken', sizes: ['s'], title: 'home.widget.packs', blurb: 'home.widget.packs.blurb', load: async () => ({
        default: () => {
          throw new Error('boom');
        },
      }),
    },
  ];
  return { WIDGETS: defs, widget: (id: string) => defs.find((def) => def.id === id) };
});

registerMessages(home);

describe('WidgetFrame', () => {
  test('a widget that throws shows a quiet tile, and its neighbours still render', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    render(
      <>
        <WidgetFrame item={{ key: 'broken-0', id: 'broken', size: 's' }} editing={false} />
        <WidgetFrame item={{ key: 'fine-1', id: 'fine', size: 's' }} editing={false} />
      </>,
    );
    expect(await screen.findByText('all good')).toBeTruthy();
    expect(await screen.findByText("This widget couldn't load.")).toBeTruthy();
    expect(screen.getByRole('region', { name: 'Packs' })).toBeTruthy();
    warn.mockRestore();
    error.mockRestore();
  });

  test('an id this client does not know shows a quiet tile', () => {
    render(<WidgetFrame item={{ key: 'gone-0', id: 'gone', size: 's' }} editing={false} />);
    expect(screen.getByText('This widget is no longer available.')).toBeTruthy();
  });
});
