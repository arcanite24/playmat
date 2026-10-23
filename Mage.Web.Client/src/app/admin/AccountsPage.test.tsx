import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { AccountRow } from '../../protocol/generated/views';
import { AccountsPage } from './AccountsPage';

describe('the account list', () => {
  it.each([true, false, undefined])('only marks email that is not confirmed (emailVerified: %s)', async (emailVerified) => {
    const account: AccountRow = { name: 'Player', email: 'player@example.com', emailVerified, active: true };
    const api = createApi({ call: vi.fn().mockResolvedValue({ accounts: [account], total: 1 }) });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <AccountsPage api={api} role="admin" />
      </QueryClientProvider>,
    );

    const email = await screen.findByText(/player@example.com/);
    expect(email.textContent).toBe(emailVerified ? 'player@example.com' : 'player@example.com · not confirmed');
  });
});
