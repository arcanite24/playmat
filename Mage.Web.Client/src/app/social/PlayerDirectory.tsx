import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { MessageCircle, Star, StarOff } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { DirectoryPlayer } from '../../protocol/generated/views';
import { api } from '../connection';
import { useLobby } from '../stores/lobby';
import { useSession } from '../stores/session';
import { isFriend, useSocial } from '../stores/social';
import { Button, IconButton } from '../ui/Button';
import { Avatar } from './Avatar';
import { LevelBadge } from './LevelBadge';
import styles from './LobbyDrawer.module.css';

const SEARCH_DELAY_MS = 300;

/**
 * Every registered player of the server, online or not (#195): find someone by name, see their level and whether they
 * are around, and add them to your friends. Friends stay this browser's list: there are no friend requests.
 */
export function PlayerDirectory() {
  const [text, setText] = useState('');
  const [query, setQuery] = useState('');
  const [offset, setOffset] = useState(0);
  const connected = useSession((state) => state.connection === 'open');

  useEffect(() => {
    const timer = setTimeout(() => {
      setQuery(text.trim());
      setOffset(0);
    }, SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  }, [text]);

  const page = useQuery({
    queryKey: ['playerDirectory', query, offset],
    queryFn: () => api.playerDirectory(query, offset),
    enabled: connected,
    staleTime: 30_000,
    placeholderData: keepPreviousData,
  });
  const data = page.data;
  const players = data?.players ?? [];
  const total = data?.total ?? 0;
  const size = data?.pageSize || 50;

  if (data && !data.enabled) {
    return <p className={styles.note}>This server doesn&rsquo;t list its players.</p>;
  }
  return (
    <div className={styles.players}>
      <label className={styles.search}>
        <span className={styles.groupLabel}>Find a player</span>
        <input
          type="search"
          value={text}
          placeholder="Name"
          maxLength={40}
          onChange={(event) => setText(event.target.value)}
          className={styles.searchInput}
        />
      </label>
      {page.error ? (
        <p className={styles.note}>The list can&rsquo;t be loaded right now.</p>
      ) : (
        <>
          <p className={styles.note} aria-live="polite">
            {data ? (total === 0 ? 'Nobody by that name.' : `${total} ${total === 1 ? 'player' : 'players'}`) : 'Loading…'}
          </p>
          <ul className={styles.list} aria-label="Registered players">
            {players.map((player) => <DirectoryRow key={player.name} player={player} />)}
          </ul>
          {total > size && (
            <div className={styles.pager}>
              <Button size="sm" variant="quiet" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - size))}>Previous</Button>
              <span className={styles.note}>{offset + 1}–{Math.min(offset + size, total)}</span>
              <Button size="sm" variant="quiet" disabled={offset + size >= total} onClick={() => setOffset(offset + size)}>Next</Button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function presenceLabel(player: DirectoryPlayer): string {
  if (!player.online) return 'Offline';
  if (player.status === 'looking') return 'Looking to play';
  if (player.status === 'hidden') return 'Hidden';
  return 'Online';
}

function DirectoryRow({ player }: { player: DirectoryPlayer }) {
  const name = player.name ?? '';
  const me = useSession((state) => state.userName);
  const friend = useSocial((state) => isFriend(state, name));
  const isMe = name.toLowerCase() === me.toLowerCase();
  return (
    <li className={[styles.player, player.online ? '' : styles.away].join(' ')}>
      <button type="button" className={styles.identity} onClick={() => useLobby.getState().openProfile(name)} title={`${name}'s profile`}>
        <Avatar name={name} size="sm" />
        <span className={styles.playerName}>{name}<LevelBadge level={player.level} />{isMe ? ' (you)' : ''}</span>
        {friend && <Star className={styles.star} size={14} aria-label="Friend" />}
      </button>
      <span className={styles.status}>
        <span className={[styles.dot, player.online ? styles.dotOn : ''].join(' ')} aria-hidden="true" />
        {presenceLabel(player)}
      </span>
      {!isMe && (
        <span className={styles.rowActions}>
          {player.online && (
            <IconButton label={`Whisper to ${name}`} icon={<MessageCircle size={16} />} onClick={() => useLobby.getState().whisper(name)} />
          )}
          <IconButton
            label={friend ? `Remove ${name} from friends` : `Add ${name} to friends`}
            icon={friend ? <StarOff size={16} /> : <Star size={16} />}
            onClick={() => (friend ? useSocial.getState().removeFriend(name) : useSocial.getState().addFriend(name))}
          />
        </span>
      )}
    </li>
  );
}
