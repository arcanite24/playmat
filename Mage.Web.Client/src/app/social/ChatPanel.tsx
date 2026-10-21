import { Send } from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { Link } from 'react-router-dom';
import { CHAT_HELP, completeWhisper, parseChatInput } from '../../core/social/chatInput';
import { splitInviteLinks, withoutIgnored, type ChatLine } from '../../core/social/chatLine';
import { useSession } from '../stores/session';
import { useSocial } from '../stores/social';
import { IconButton } from '../ui/Button';
import { PromptText } from '../ui/PromptText';
import { Flair } from '../support/Flair';
import styles from './ChatPanel.module.css';
import { PUBLIC_ORIGIN } from '../brand';

export interface ChatPanelProps {
  /** names the box offers for "/w" completion */
  names?: readonly string[];
  lines: readonly ChatLine[];
  ready: boolean;
  onSend(text: string): Promise<void> | void;
  /** accessible name of the chat ("Lobby chat") */
  label: string;
  placeholder?: string;
  empty?: string;
  /** text handed in from outside (a whisper started from the player list) */
  incoming?: string | null;
  /** the incoming text is in the box now */
  onIncomingTaken?(): void;
  /** lines only this player sees (help, the ignore list); with none given, the panel keeps its own */
  onLocal?(text: string): void;
  className?: string;
}

let localSeq = 0;

/** A chat: lines, whispers, links to invites, and a box that understands /help, /w, /ignore and /friend. */
export function ChatPanel({ names = [], lines, ready, onSend, label, placeholder, empty, incoming, onIncomingTaken, onLocal, className }: ChatPanelProps) {
  const ignored = useSocial((state) => state.ignored);
  const myName = useSession((state) => state.userName);
  const [draft, setDraft] = useState('');
  const [ownLocal, setOwnLocal] = useState<ChatLine[]>([]);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLOListElement>(null);

  const addLocal = (text: string) => {
    if (onLocal) {
      onLocal(text);
      return;
    }
    const line: ChatLine = { key: `local${localSeq++}`, at: Date.now(), kind: 'info', who: null, segments: [{ kind: 'text', text }], text };
    setOwnLocal((current) => [...current, line].slice(-50));
  };

  const visible = useMemo(() => {
    const merged = ownLocal.length > 0 ? [...lines, ...ownLocal].sort((a, b) => a.at - b.at) : lines;
    return withoutIgnored(merged, ignored);
  }, [lines, ownLocal, ignored]);

  useEffect(() => {
    list.current?.scrollTo({ top: list.current.scrollHeight });
  }, [visible.length]);

  // a whisper started elsewhere lands in the box, ready to type
  useEffect(() => {
    if (!incoming) return;
    setDraft(incoming);
    input.current?.focus();
    onIncomingTaken?.();
  }, [incoming, onIncomingTaken]);

  const startWhisper = (name: string) => {
    setDraft(`/w ${name} `);
    input.current?.focus();
  };

  const submit = () => {
    const parsed = parseChatInput(draft);
    if (!parsed) return;
    const social = useSocial.getState();
    switch (parsed.kind) {
      case 'send':
        void Promise.resolve(onSend(parsed.text)).catch(() => addLocal("The message didn't go through."));
        // keep "/w name " so a whispered conversation can go on
        setDraft(/^[/\\](w|whisper)\s+\S+\s/i.test(parsed.text) ? parsed.text.replace(/^([/\\]\w+\s+\S+\s+).*$/s, '$1') : '');
        return;
      case 'help':
        addLocal('Chat commands:');
        CHAT_HELP.forEach((item) => addLocal(`${item.command} — ${item.detail}`));
        break;
      case 'ignoreList':
        addLocal(social.ignored.length > 0 ? `You ignore: ${social.ignored.join(', ')}` : "You don't ignore anyone.");
        break;
      case 'ignore':
        if (parsed.name.toLowerCase() === myName.toLowerCase()) {
          addLocal("You can't ignore yourself.");
          break;
        }
        social.ignore(parsed.name);
        addLocal(`Ignoring ${parsed.name}: you won't see their chat or whispers, and they can't join your tables.`);
        break;
      case 'unignore':
        social.unignore(parsed.name);
        addLocal(`No longer ignoring ${parsed.name}.`);
        break;
      case 'friend':
        if (parsed.name.toLowerCase() === myName.toLowerCase()) {
          addLocal('You are already your own best friend.');
          break;
        }
        social.addFriend(parsed.name);
        addLocal(`${parsed.name} is a friend: you'll hear when they come online.`);
        break;
      case 'unfriend':
        social.removeFriend(parsed.name);
        addLocal(`${parsed.name} is no longer on your friends list.`);
        break;
      case 'error':
        addLocal(parsed.message);
        return;
    }
    setDraft('');
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== 'Tab' || event.shiftKey) return;
    const completed = completeWhisper(draft, names.filter((name) => name !== myName));
    if (completed) {
      event.preventDefault();
      setDraft(completed);
    }
  };

  return (
    <section className={[styles.chat, className ?? ''].join(' ')} aria-label={label}>
      <ol ref={list} className={styles.lines} aria-live="polite">
        {visible.length === 0 && <li className={styles.empty}>{empty ?? 'No messages yet. Type /help for commands.'}</li>}
        {visible.map((line) => <Line key={line.key} line={line} me={myName} onWhisper={startWhisper} />)}
      </ol>
      <form
        className={styles.form}
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <input
          ref={input}
          className={styles.input}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={ready ? placeholder ?? 'Say something, or /help' : 'Joining chat…'}
          disabled={!ready}
          aria-label={`${label} message`}
          maxLength={400}
        />
        <IconButton label="Send" icon={<Send size={16} />} type="submit" disabled={!ready || !draft.trim()} />
      </form>
    </section>
  );
}

function Line({ line, me, onWhisper }: { line: ChatLine; me: string; onWhisper(name: string): void }) {
  const kindClass = styles[line.kind] ?? '';
  const time = new Date(line.at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  if (line.kind === 'info' || line.kind === 'status' || line.kind === 'game') {
    return (
      <li className={[styles.line, kindClass].join(' ')}>
        {line.segments.map((segment, index) => (segment.kind === 'text'
          ? <PromptText key={index} text={segment.text} />
          : <span key={index} className={styles.name}>{segment.name}</span>))}
      </li>
    );
  }
  const other = line.who ?? '';
  const prefix = line.kind === 'whisperFrom' ? 'from ' : line.kind === 'whisperTo' ? 'to ' : '';
  return (
    <li className={[styles.line, kindClass, line.who === me ? styles.mine : ''].join(' ')}>
      <time className={styles.time} dateTime={new Date(line.at).toISOString()}>{time}</time>
      {prefix && <span className={styles.whisperTag}>{prefix}</span>}
      {other && other !== me
        ? <button type="button" className={styles.who} onClick={() => onWhisper(other)} title={`Whisper to ${other}`}>{other}</button>
        : <span className={styles.who}>{other || 'You'}</span>}
      <Flair name={other || me} />
      <span className={styles.text}>
        {splitInviteLinks(line.text, PUBLIC_ORIGIN).map((part, index) => (part.kind === 'invite'
          ? <Link key={index} className={styles.invite} to={part.path}>Join the table</Link>
          : <span key={index}>{part.text}</span>))}
      </span>
    </li>
  );
}
