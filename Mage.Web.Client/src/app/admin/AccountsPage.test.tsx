import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { AccountRow } from '../../protocol/generated/views';
import { i18nReady, useI18n } from '../i18n';
import core from '../i18n/en/core';
import es from '../i18n/es';
import { createTranslator } from '../i18n/translate';
import { AccountsPage } from './AccountsPage';

const initialI18n = useI18n.getState();

afterEach(() => useI18n.setState(initialI18n));

describe.each([
  ['en', core, 'not confirmed'],
  ['es', es, 'sin confirmar'],
] as const)('the account list in %s', (locale, messages, notConfirmed) => {
  beforeEach(async () => {
    await i18nReady;
    useI18n.setState({ locale, t: createTranslator({ locale, messages, fallback: core }) });
  });

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
    expect(email.textContent).toBe(emailVerified ? 'player@example.com' : `player@example.com · ${notConfirmed}`);
  });
});
