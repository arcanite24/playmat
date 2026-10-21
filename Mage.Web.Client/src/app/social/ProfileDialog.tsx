import { useQuery } from '@tanstack/react-query';
import { Ban, Flag as FlagIcon, History, MessageCircle, Star, StarOff } from 'lucide-react';
import { lazy, Suspense, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { DEFAULT_AVATAR_ID, flagChoices, SIGILS } from '../../core/social/identity';
import { api } from '../connection';
import { useT } from '../i18n';
import { useServerInfo } from '../queries';
import { useLobby } from '../stores/lobby';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { isFriend, isIgnored, useSocial } from '../stores/social';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Avatar, Flag } from './Avatar';
import { Flair } from '../support/Flair';
import { useLook } from '../support/supportData';
import { THANK_YOU } from '../support/thankYou';
import { titleColor, titleName } from '../career/progressModel';
import { LevelBadge } from './LevelBadge';
import { gameKind, levelShare, recentGames, shownLevel } from './progressionModel';
import styles from './ProfileDialog.module.css';

// your Career line loads with the Career screens, for players who opted in
const CareerSummary = lazy(() => import('../career/CareerScreen').then((m) => ({ default: m.CareerSummary })));
// achievement points and the badges a player shows (B307), with the Career code
const CareerShowcase = lazy(() => import('../career/ShowcaseBadges'));

/** A player's card: ratings, match and event record, what they're doing now; your own sigil and flag. */
export function ProfileDialog() {
  const name = useLobby((state) => state.profile);
  const close = () => useLobby.getState().openProfile(null);
  return (
    <Dialog open={!!name} onOpenChange={(open) => !open && close()} title={name ? `${name}` : 'Profile'} width="md">
      {name && <Profile name={name} onClose={close} />}
    </Dialog>
  );
}

function Profile({ name, onClose }: { name: string; onClose(): void }) {
  const navigate = useNavigate();
  const me = useSession((state) => state.userName);
  const isMe = name === me;
  const user = useLobby((state) => state.users.find((candidate) => candidate.userName === name) ?? null);
  const settings = useSettings((state) => state.settings);
  const friend = useSocial((state) => isFriend(state, name));
  const ignoredUser = useSocial((state) => isIgnored(state, name));
  const career = useSettings((state) => state.settings.careerOptIn);
  // your own choice shows at once; the server's copy catches up on the next look
  const avatarId = isMe ? settings.avatarId : user?.avatarId;
  const flagName = isMe ? `${settings.flag}.png` : user?.flagName;
  const look = useLook(name);

  const stats = [
    { label: 'Rating', value: user?.generalRating },
    { label: 'Constructed', value: user?.constructedRating },
    { label: 'Limited', value: user?.limitedRating },
  ];

  return (
    <div className={styles.profile}>
      <header className={styles.head}>
        <Avatar name={name} avatarId={avatarId} size="lg" ring={look?.frame === THANK_YOU.frame} frame={look?.frame} />
        <div>
          <p className={styles.name}>{name}<LevelBadge level={look?.level} /><Flair name={name} /> <Flag flagName={flagName} always /></p>
          {look?.title && <p className={styles.title} style={{ color: titleColor(look.title) }}>{titleName(look.title)}</p>}
          <p className={styles.presence}>{user ? (user.infoGames?.trim() || 'Online') : 'Offline'}</p>
        </div>
      </header>

      {user ? (
        <>
          <dl className={styles.ratings}>
            {stats.map((stat) => (
              <div key={stat.label} className={styles.rating}>
                <dt>{stat.label}</dt>
                <dd>{stat.value ? Math.round(stat.value) : '—'}</dd>
              </div>
            ))}
          </dl>
          <dl className={styles.record}>
            <dt>Matches</dt>
            <dd>{user.matchHistory?.trim() || 'None yet'}{quit(user.matchQuitRatio)}</dd>
            <dt>Events</dt>
            <dd>{user.tourneyHistory?.trim() || 'None yet'}{quit(user.tourneyQuitRatio)}</dd>
          </dl>
        </>
      ) : (
        <p className={styles.note}>{name} is not online, so their record isn't available right now.</p>
      )}

      <Progression name={name} />

      {isMe ? <Identity /> : (
        <div className={styles.actions}>
          {user && (
            <Button size="sm" icon={<MessageCircle size={16} />} onClick={() => { onClose(); useLobby.getState().whisper(name); }}>
              Whisper
            </Button>
          )}
          <Button
            size="sm"
            variant="quiet"
            icon={friend ? <StarOff size={16} /> : <Star size={16} />}
            onClick={() => (friend ? useSocial.getState().removeFriend(name) : useSocial.getState().addFriend(name))}
          >
            {friend ? 'Remove friend' : 'Add friend'}
          </Button>
          <Button
            size="sm"
            variant={ignoredUser ? 'quiet' : 'danger'}
            icon={<Ban size={16} />}
            onClick={() => (ignoredUser ? useSocial.getState().unignore(name) : useSocial.getState().ignore(name))}
          >
            {ignoredUser ? 'Stop ignoring' : 'Ignore'}
          </Button>
          <Button size="sm" variant="quiet" icon={<FlagIcon size={16} />} onClick={() => { onClose(); useLobby.getState().openReport(name); }}>
            Report
          </Button>
        </div>
      )}
      {isMe && career && <Suspense fallback={null}><CareerSummary user={name} onOpen={onClose} /></Suspense>}
      {isMe && (
        <div className={styles.actions}>
          <Button size="sm" variant="quiet" icon={<History size={16} />} onClick={() => { onClose(); navigate('/history'); }}>
            Your matches and replays
          </Button>
        </div>
      )}
    </div>
  );
}

/**
 * The player's level, XP into it, achievements, recent record and last counted games (servers with Career). Nothing
 * when they have none, hid it, or the server doesn't keep Career.
 */
function Progression({ name }: { name: string }) {
  const t = useT();
  const accounts = useServerInfo().data?.accounts === true;
  const me = useSession((state) => state.userName.toLowerCase());
  const profile = useQuery({
    queryKey: ['careerProfileOf', me, name.toLowerCase()],
    queryFn: () => api.careerProfileOf(name),
    enabled: accounts,
    staleTime: 60_000,
    retry: false,
  });
  const badges = useLook(name)?.badges;
  const data = profile.data;
  const level = shownLevel(data?.level);
  if (!data || level === null) return null;
  const share = levelShare(data.levelXp, data.levelSize);
  const recent = recentGames(data.recent, 5);
  return (
    <section className={styles.progress} aria-label={t('social.progress.title')}>
      <div className={styles.levelRow}>
        <span className={styles.level}>{t('social.progress.level', { level })}</span>
        {share !== null && (
          <span className={styles.xpText}>{t('social.progress.xp', { xp: data.levelXp ?? 0, size: data.levelSize ?? 0, next: level + 1 })}</span>
        )}
      </div>
      {share !== null && (
        <div
          className={styles.xpBar}
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={data.levelSize}
          aria-valuenow={Math.min(data.levelXp ?? 0, data.levelSize ?? 0)}
          aria-label={t('social.progress.xpAria', { next: level + 1 })}
        >
          <span style={{ width: `${Math.round(share * 100)}%` }} />
        </div>
      )}
      {(!!data.points || (badges?.length ?? 0) > 0 || name.toLowerCase() === me) && (
        <Suspense fallback={null}><CareerShowcase name={name} points={data.points} rank={data.rank} badges={badges} /></Suspense>
      )}
      <dl className={styles.record}>
        {data.achievementsTotal ? (
          <>
            <dt>{t('social.progress.achievements')}</dt>
            <dd>{t('social.progress.achievementsValue', { earned: data.achievements ?? 0, total: data.achievementsTotal })}</dd>
          </>
        ) : null}
        {data.games ? (
          <>
            <dt>{t('social.progress.record')}</dt>
            <dd>{t('social.progress.recordValue', { wins: data.wins ?? 0, count: data.games })}</dd>
          </>
        ) : null}
      </dl>
      {recent.length > 0 && (
        <>
          <h3 className={styles.recentTitle}>{t('social.progress.recent')}</h3>
          <ul className={styles.recent}>
            {recent.map((game, index) => (
              <li key={game.gameKey ?? index} className={game.revoked ? styles.revoked : undefined}>
                <span className={game.won ? styles.won : styles.lost}>{t(game.won ? 'social.progress.won' : 'social.progress.lost')}</span>
                <span className={styles.kind}>{t(`social.progress.kind.${gameKind(game.kind)}`)}</span>
                <span className={styles.gain}>
                  {game.revoked ? t('social.progress.revoked') : t('social.progress.gain', { xp: game.xp ?? 0 })}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

function quit(ratio: number | undefined): string {
  return ratio ? ` · left ${ratio}%` : '';
}

/** Your own sigil and flag, sent to the server with your other preferences. */
function Identity() {
  const avatarId = useSettings((state) => state.settings.avatarId);
  const flag = useSettings((state) => state.settings.flag);
  const update = useSettings((state) => state.update);
  const me = useSession((state) => state.userName);
  const flags = useMemo(() => flagChoices(), []);
  return (
    <div className={styles.identity}>
      <fieldset className={styles.sigils}>
        <legend>Sigil</legend>
        <label className={styles.sigil} title="Your initial">
          <input type="radio" name="sigil" checked={avatarId === DEFAULT_AVATAR_ID} onChange={() => update({ avatarId: DEFAULT_AVATAR_ID })} />
          <Avatar name={me} size="md" />
          <span className="visually-hidden">Your initial</span>
        </label>
        {SIGILS.map((sigil) => (
          <label key={sigil.id} className={styles.sigil} title={sigil.label}>
            <input type="radio" name="sigil" checked={avatarId === sigil.id} onChange={() => update({ avatarId: sigil.id })} />
            <Avatar name={me} avatarId={sigil.id} size="md" />
            <span className="visually-hidden">{sigil.label}</span>
          </label>
        ))}
      </fieldset>
      <label className={styles.flagField}>
        <span>Flag</span>
        <select value={flag} onChange={(event) => update({ flag: event.target.value })}>
          {flags.map((choice) => <option key={choice.code} value={choice.code}>{choice.glyph} {choice.label}</option>)}
        </select>
      </label>
    </div>
  );
}
