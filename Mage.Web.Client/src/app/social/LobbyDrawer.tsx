import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { Ban, Flag as FlagIcon, MessageCircle, MoreHorizontal, Send, Star, StarOff, UserRound, X } from 'lucide-react';
import { useEffect, useMemo } from 'react';
import { inviteLink, toChatLines, type ChatLine } from '../../core/social/chatLine';
import type { TableView, UsersView } from '../../protocol/generated/views';
import { useTables } from '../queries';
import { useLobby, type LobbyMessage } from '../stores/lobby';
import { useSession } from '../stores/session';
import { isFriend, isIgnored, useSocial } from '../stores/social';
import { notify } from '../stores/toasts';
import { IconButton } from '../ui/Button';
import { Avatar, Flag } from './Avatar';
import { ChatPanel } from './ChatPanel';
import { LevelBadge, NameLevel } from './LevelBadge';
import { Flair } from '../support/Flair';
import { useLook } from '../support/supportData';
import { THANK_YOU } from '../support/thankYou';
import shell from '../AppShell.module.css';
import styles from './LobbyDrawer.module.css';
import { PUBLIC_ORIGIN } from '../brand';

/** The lobby: the main room's chat and the players online, beside whatever screen is open. */
export function LobbyDrawer() {
  const open = useLobby((state) => state.open);
  const setOpen = useLobby((state) => state.setOpen);
  const tab = useLobby((state) => state.tab);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !event.defaultPrevented) setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, setOpen]);

  if (!open) return null;
  return (
    <aside className={styles.drawer} aria-label="Lobby">
      <header className={styles.head}>
        <div className={styles.tabs} role="tablist" aria-label="Lobby">
          <Tab id="chat" label="Chat" />
          <Tab id="players" label="Players" />
        </div>
        <IconButton label="Close lobby" icon={<X size={18} />} onClick={() => setOpen(false)} />
      </header>
      <div className={styles.body} role="tabpanel" aria-label={tab === 'chat' ? 'Lobby chat' : 'Players online'}>
        {tab === 'chat' ? <LobbyChat /> : <PlayerList />}
      </div>
    </aside>
  );
}

function Tab({ id, label }: { id: 'chat' | 'players'; label: string }) {
  const tab = useLobby((state) => state.tab);
  const setOpen = useLobby((state) => state.setOpen);
  const count = useLobby((state) => (id === 'players' ? state.users.length : 0));
  return (
    <button type="button" role="tab" aria-selected={tab === id} className={styles.tab} onClick={() => setOpen(true, id)}>
      {label}{count > 0 ? ` (${count})` : ''}
    </button>
  );
}

const takeLobbyDraft = () => {
  useLobby.getState().takeDraft();
};

/** lobby messages become lines once each, however often the list re-renders */
const parsedMessages = new WeakMap<LobbyMessage, ChatLine[]>();

function linesOf(messages: readonly LobbyMessage[]): ChatLine[] {
  return messages.flatMap((entry) => {
    let lines = parsedMessages.get(entry);
    if (!lines) {
      lines = toChatLines(entry.message, entry.key);
      parsedMessages.set(entry, lines);
    }
    return lines;
  });
}

function LobbyChat() {
  const messages = useLobby((state) => state.messages);
  const lines = useMemo(() => linesOf(messages), [messages]);
  const ready = useLobby((state) => !!state.chatId);
  const users = useLobby((state) => state.users);
  const incoming = useLobby((state) => state.draft);
  const names = useMemo(() => users.map((user) => user.userName ?? '').filter(Boolean), [users]);
  return (
    <ChatPanel
      label="Lobby chat"
      lines={lines}
      ready={ready}
      names={names}
      onSend={(text) => useLobby.getState().send(text)}
      incoming={incoming}
      onIncomingTaken={takeLobbyDraft}
      onLocal={(text) => useLobby.getState().addLocal(text)}
      empty="Nobody has said anything yet. Say hello, or type /help."
    />
  );
}

/** My tables still waiting for players, to invite someone to. */
function useMyOpenTables(): TableView[] {
  const { data: tables = [] } = useTables();
  const me = useSession((state) => state.userName);
  return useMemo(
    () => tables.filter((table) => !table.isTournament && table.tableState === 'WAITING' && table.seats?.some((seat) => seat.playerName === me)),
    [tables, me],
  );
}

function PlayerList() {
  const users = useLobby((state) => state.users);
  const me = useSession((state) => state.userName);
  const friends = useSocial((state) => state.friends);
  const ignored = useSocial((state) => state.ignored);
  const myTables = useMyOpenTables();
  const sorted = useMemo(() => {
    const social = { friends, ignored };
    const rank = (user: UsersView) => (user.userName === me ? 0 : isFriend(social, user.userName) ? 1 : isIgnored(social, user.userName) ? 3 : 2);
    return [...users].sort((a, b) => rank(a) - rank(b) || (a.userName ?? '').localeCompare(b.userName ?? '', undefined, { sensitivity: 'base' }));
  }, [users, friends, ignored, me]);
  const offlineFriends = friends.filter((name) => !users.some((user) => user.userName?.toLowerCase() === name.toLowerCase()));

  return (
    <div className={styles.players}>
      <ul className={styles.list} aria-label="Players online">
        {sorted.map((user) => <PlayerRow key={user.userName} user={user} me={me} myTables={myTables} />)}
      </ul>
      {offlineFriends.length > 0 && (
        <>
          <h3 className={styles.groupLabel}>Friends offline</h3>
          <ul className={styles.list} aria-label="Friends offline">
            {offlineFriends.map((name) => (
              <li key={name} className={[styles.player, styles.offline].join(' ')}>
                <Avatar name={name} size="sm" />
                <span className={styles.playerName}>{name}<NameLevel name={name} /></span>
                <IconButton label={`Remove ${name} from friends`} icon={<StarOff size={16} />} onClick={() => useSocial.getState().removeFriend(name)} />
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

function PlayerRow({ user, me, myTables }: { user: UsersView; me: string; myTables: TableView[] }) {
  const name = user.userName ?? '';
  const friend = useSocial((state) => isFriend(state, name));
  const ignoredUser = useSocial((state) => isIgnored(state, name));
  const isMe = name === me;
  const look = useLook(name);
  const status = (user.infoGames ?? '').trim();

  const invite = (table: TableView) => {
    if (!table.tableId) return;
    const link = inviteLink(PUBLIC_ORIGIN, table.tableId);
    void useLobby.getState().send(`/w ${name} Join my table ${table.tableName ?? ''}: ${link}`)
      .then(() => notify('Invite sent', `${name} got a link to ${table.tableName}.`))
      .catch(() => notify("Couldn't send the invite", 'The lobby chat is not connected.', 'error'));
  };

  return (
    <li className={[styles.player, ignoredUser ? styles.ignored : ''].join(' ')}>
      <button type="button" className={styles.identity} onClick={() => useLobby.getState().openProfile(name)} title={`${name}'s profile`}>
        <Avatar name={name} avatarId={user.avatarId} size="sm" ring={look?.frame === THANK_YOU.frame} />
        <span className={styles.playerName}>
          {name}<LevelBadge level={look?.level} /><Flair name={name} />{isMe ? ' (you)' : ''} <Flag flagName={user.flagName} />
        </span>
        {friend && <Star className={styles.star} size={14} aria-label="Friend" />}
      </button>
      {status && <span className={styles.status} title={status}>{status}</span>}
      {!isMe && (
        <DropdownMenu.Root>
          <DropdownMenu.Trigger asChild>
            <IconButton label={`Actions for ${name}`} icon={<MoreHorizontal size={16} />} />
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content className={shell.menu} align="end" sideOffset={6}>
              <DropdownMenu.Item className={shell.menuItem} onSelect={() => useLobby.getState().whisper(name)}>
                <MessageCircle size={16} aria-hidden="true" /> Whisper
              </DropdownMenu.Item>
              {myTables.map((table) => (
                <DropdownMenu.Item key={table.tableId} className={shell.menuItem} onSelect={() => invite(table)}>
                  <Send size={16} aria-hidden="true" /> Invite to {table.tableName}
                </DropdownMenu.Item>
              ))}
              {myTables.length === 0 && (
                <DropdownMenu.Item className={shell.menuItem} disabled>
                  <Send size={16} aria-hidden="true" /> Host a table to invite
                </DropdownMenu.Item>
              )}
              <DropdownMenu.Item className={shell.menuItem} onSelect={() => useLobby.getState().openProfile(name)}>
                <UserRound size={16} aria-hidden="true" /> Profile
              </DropdownMenu.Item>
              <DropdownMenu.Separator className={shell.menuRule} />
              <DropdownMenu.Item
                className={shell.menuItem}
                onSelect={() => (friend ? useSocial.getState().removeFriend(name) : useSocial.getState().addFriend(name))}
              >
                {friend ? <StarOff size={16} aria-hidden="true" /> : <Star size={16} aria-hidden="true" />} {friend ? 'Remove friend' : 'Add friend'}
              </DropdownMenu.Item>
              <DropdownMenu.Item
                className={shell.menuItem}
                onSelect={() => (ignoredUser ? useSocial.getState().unignore(name) : useSocial.getState().ignore(name))}
              >
                <Ban size={16} aria-hidden="true" /> {ignoredUser ? 'Stop ignoring' : 'Ignore'}
              </DropdownMenu.Item>
              <DropdownMenu.Item className={shell.menuItem} onSelect={() => useLobby.getState().openReport(name)}>
                <FlagIcon size={16} aria-hidden="true" /> Report
              </DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
      )}
    </li>
  );
}
