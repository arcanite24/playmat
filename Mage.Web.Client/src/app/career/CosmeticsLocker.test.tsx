import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, test } from 'vitest';
import { loadTestCatalog } from '../../test/catalog';
import { queryClient } from '../queries';
import { useSession } from '../stores/session';
import { useCareerLook } from './careerData';
import { CosmeticsLocker } from './CosmeticsLocker';

loadTestCatalog();

function renderLocker() {
  useSession.setState({ userName: 'tester' });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <CosmeticsLocker
          track={{ unlocks: [{ kind: 'avatar', id: 'owl' }, { kind: 'frame', id: 'frame-bronze' }] }}
          achievements={[]}
        />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe('the locker', () => {
  test('groups the catalog by kind, picks owned pieces and says where locked ones come from', () => {
    renderLocker();
    const tabs = screen.getByRole('tablist', { name: 'Locker sections' });
    // the bronze frame is new (the avatars' tab is open, so it has no dot)
    expect(within(tabs).getByRole('img', { name: 'New in Frames' })).toBeTruthy();
    expect(within(tabs).queryByRole('img', { name: 'New in Avatars' })).toBeNull();
    fireEvent.click(screen.getByRole('radio', { name: /Owl/ }));
    expect(useCareerLook.getState().looks.tester?.avatar).toBe('owl');
    expect((screen.getByRole('radio', { name: /Fox/ }) as HTMLButtonElement).disabled).toBe(true);

    fireEvent.click(within(tabs).getByRole('tab', { name: /Frames/ }));
    fireEvent.click(screen.getByRole('radio', { name: /Bronze/ }));
    expect(useCareerLook.getState().looks.tester?.frame).toBe('frame-bronze');

    fireEvent.click(within(tabs).getByRole('tab', { name: /Emotes/ }));
    // the free phrases are owned; the rest say where they come from (the raw catalog file has no sources yet)
    expect(screen.getByText('Table talk')).toBeTruthy();
    expect(screen.getByText('Hello!')).toBeTruthy();
    expect(screen.getAllByText(/Earned in Career/).length).toBe(5);
  });
});
