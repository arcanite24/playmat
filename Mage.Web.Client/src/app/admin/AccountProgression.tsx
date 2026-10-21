import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { PaidGame } from '../../protocol/generated/views';
import { Button } from '../ui/Button';
import { ConfirmAction } from './ConfirmAction';
import { atLeast, describeAge, type StaffRole } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

/**
 * An account's progression for staff (B272): level, XP, coins, the games that paid it and why, and the people it hit
 * the daily pair cap with. Admins can take back what a game paid, with a reason for the audit log.
 */
export function AccountProgression({ api, role, name, now }: { api: AdminApi; role: StaffRole; name: string; now: number }) {
  const client = useQueryClient();
  const view = useQuery({ queryKey: ['admin', 'career', name], queryFn: () => api.adminCareer(name), retry: false });
  const [revoking, setRevoking] = useState<PaidGame | null>(null);
  const admin = atLeast(role, 'admin');
  // a server without Career has nothing to show
  if (view.isError || view.isPending) return null;
  const data = view.data;
  const capped = Object.entries(data?.cappedPairs ?? {});

  return (
    <section className={styles.block} aria-labelledby="account-progression">
      <h2 id="account-progression">Progression</h2>
      {!data ? <p className={styles.quiet}>{name} hasn't played a counted game yet.</p> : (
        <>
          <p>Level {data.level} · {data.xp} XP · {data.coins} coins · {data.collection ? 'has a Career collection' : 'no Career collection'}</p>
          {capped.length > 0 && (
            <p className={styles.quiet}>Hit the daily pair cap with {capped.map(([pair, days]) => `${pair.split('|').find((who) => who !== name.toLowerCase()) ?? pair} (${days} ${days === 1 ? 'day' : 'days'})`).join(', ')} in the last 14 days.</p>
          )}
          {revoking && (
            <ConfirmAction title={`Take back game ${revoking.gameKey}`}
              what={`${name} loses the ${revoking.xp} XP and ${revoking.coins} coins it paid. Quests and achievements it finished stay.`}
              confirmLabel="Take it back"
              onCancel={() => setRevoking(null)} onConfirm={async (reason) => {
                await api.adminCareerRevoke(name, revoking.gameKey ?? '', reason);
                setRevoking(null);
                await client.invalidateQueries({ queryKey: ['admin', 'career', name] });
                await client.invalidateQueries({ queryKey: ['admin', 'account', name] });
              }} />
          )}
          {(data.games?.length ?? 0) === 0 ? <p className={styles.quiet}>No counted games yet.</p> : (
            <table className={styles.table}>
              <thead><tr><th>When</th><th>Kind</th><th>Result</th><th>XP</th><th>Coins</th><th>Why</th>{admin && <th />}</tr></thead>
              <tbody>
                {data.games!.map((game) => (
                  <tr key={game.gameKey}>
                    <td>{describeAge(game.at ?? 0, now)}</td>
                    <td>{game.kind}</td>
                    <td>{game.won ? 'Won' : 'Lost'}</td>
                    <td>{game.xp}</td>
                    <td>{game.coins}</td>
                    <td className={styles.clip}>{game.revoked ? 'Taken back' : game.reason}</td>
                    {admin && (
                      <td>{!game.revoked && ((game.xp ?? 0) > 0 || (game.coins ?? 0) > 0) && (
                        <Button size="sm" variant="quiet" onClick={() => setRevoking(game)}>Take back</Button>
                      )}</td>
                    )}
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </>
      )}
    </section>
  );
}
