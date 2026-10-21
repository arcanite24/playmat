import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { WhatsNewDialog } from './WhatsNewDialog';

describe('WhatsNewDialog', () => {
  test('lists the changes under translated headings, links the release and closes', () => {
    const onClose = vi.fn();
    render(<WhatsNewDialog onClose={onClose} notes={{
      version: 'v1.3.0',
      url: 'https://github.com/arcanite24/playmat/releases/tag/v1.3.0',
      sections: [
        { id: 'features', title: 'Features', items: ['Sealed events'] },
        { title: 'Highlights', items: ['A new playmat'] },
      ],
    }} />);
    expect(screen.getByRole('dialog', { name: 'What’s new in v1.3.0' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'New' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Highlights' })).toBeTruthy();
    expect(screen.getByText('Sealed events')).toBeTruthy();
    expect(screen.getByRole('link', { name: 'All changes' }).getAttribute('href')).toBe('https://github.com/arcanite24/playmat/releases/tag/v1.3.0');
    fireEvent.click(screen.getByRole('button', { name: 'Got it' }));
    expect(onClose).toHaveBeenCalled();
  });
});
