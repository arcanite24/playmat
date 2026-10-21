import { Lock } from 'lucide-react';
import { useMemo } from 'react';
import type { CareerAchievement, CareerCosmetic, CareerLevelReward, CareerLevelTrack } from '../../protocol/generated/views';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { MAT_CLOTHS, clothSwatch } from '../match/playmats';
import { useServerInfo } from '../queries';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { useThankYou } from '../support/supportData';
import { CardFace } from '../ui/CardFace';
import { Art } from './Art';
import { useCareerAchievements, useCareerLevels, useCareerLook } from './careerData';
import { CareerAvatar } from './CareerScreen';
import { CAREER_AVATARS, CAREER_PLAYMATS, CAREER_SLEEVES, cosmeticName, hasUnlock, titleLabel, unlockedIds } from './progressModel';
import { PatronPlaymat, PatronTitle, PetShelf, SupporterMarks, SupporterSleeveItem } from './SupporterLocker';
import progress from './Progress.module.css';

registerMessages(messages);

/** Where a cosmetic comes from: a level, or an achievement (secret ones stay secret). */
function sourceOf(t: ReturnType<typeof useT>, rewards: readonly CareerLevelReward[] | undefined, achievements: readonly CareerAchievement[], cosmetic: CareerCosmetic): string {
  // no Career yet: the track isn't known, only that Career gives it
  if (!rewards) return t('career.look.inCareer');
  const level = rewards?.find((reward) => reward.cosmetic?.kind === cosmetic.kind && reward.cosmetic?.id === cosmetic.id)?.level;
  if (level) return t('career.look.atLevel', { level });
  const achievement = achievements.find((item) => item.cosmetic?.kind === cosmetic.kind && item.cosmetic?.id === cosmetic.id);
  return achievement ? t('career.look.fromAchievement', { name: achievement.name ?? t('career.achievements.secret') }) : '';
}

/**
 * The locker: every cosmetic a player can wear, in every mode (the look goes to the server, so opponents see it).
 * Career's avatars, titles, sleeves and playmats, unlocked by playing it, and the supporters' thank-you set, which
 * everyone sees and only supporters can pick. Career's Cosmetics tab and Settings, Cosmetics both show it; without a
 * Career track (not started, or a server without Career) its pieces show as Career's, or not at all.
 */
export function CosmeticsLocker({ track, achievements, career = true }: {
  track: CareerLevelTrack | null | undefined;
  achievements: readonly CareerAchievement[];
  /** whether the server offers Career: without it, only the thank-you set shows */
  career?: boolean;
}) {
  const t = useT();
  const user = useSession((session) => session.userName);
  const unlocks = useMemo(() => track?.unlocks ?? [], [track?.unlocks]);
  const avatar = useCareerLook((look) => look.looks[user.toLowerCase()]?.avatar ?? null);
  const title = useCareerLook((look) => look.looks[user.toLowerCase()]?.title ?? null);
  const choose = useCareerLook((look) => look.choose);
  const cloth = useSettings((settings) => settings.settings.matCloth);
  const titles = useMemo(() => {
    // every title the content offers, unlocked ones first
    const all = new Set([...unlockedIds(unlocks, 'title'), ...achievements.flatMap((item) => (item.cosmetic?.kind === 'title' && item.cosmetic.id ? [item.cosmetic.id] : []))]);
    return [...all];
  }, [unlocks, achievements]);
  const how = (cosmetic: CareerCosmetic) => sourceOf(t, track?.rewards, achievements, cosmetic);
  // the thank-you set: a supporter's Patron title is worn instead of the Career one
  const thanks = useThankYou();
  const patron = thanks.has('title') && thanks.wear.title;
  const pickTitle = (id: string | null) => {
    choose(user, { title: id });
    if (patron) thanks.choose({ title: false });
  };

  return (
    <div className={progress.locker}>
      <section className={progress.shelfPanel} aria-labelledby="locker-avatar">
        <h3 id="locker-avatar">{t('career.look.avatar')}</h3>
        <div className={progress.options} role="radiogroup" aria-labelledby="locker-avatar">
          <LockerItem on={!avatar} open label={t('career.look.none')} onPick={() => choose(user, { avatar: null })}>
            <CareerAvatar id={null} name={user} size={64} />
          </LockerItem>
          {career && Object.keys(CAREER_AVATARS).map((id) => {
            const cosmetic = { kind: 'avatar', id };
            const open = hasUnlock(unlocks, 'avatar', id);
            return (
              <LockerItem key={id} on={avatar === id} open={open} label={open ? cosmeticName(t, cosmetic) : how(cosmetic)} onPick={() => choose(user, { avatar: id })}>
                <CareerAvatar id={id} name={user} size={64} />
              </LockerItem>
            );
          })}
        </div>
      </section>

      <section className={progress.shelfPanel} aria-labelledby="locker-title">
        <h3 id="locker-title">{t('career.look.title')}</h3>
        <p className={progress.namePlate}><b>{user}</b>{patron ? <small>{t('support.locker.patron')}</small> : title && <small>{titleLabel(title)}</small>}</p>
        <div className={progress.titles} role="radiogroup" aria-labelledby="locker-title">
          <button type="button" role="radio" aria-checked={!patron && !title} className={progress.titleOption} onClick={() => pickTitle(null)} data-nav>
            {t('career.look.none')}
          </button>
          {titles.map((id) => {
            const cosmetic = { kind: 'title', id };
            const open = hasUnlock(unlocks, 'title', id);
            const from = how(cosmetic);
            return (
              <button
                key={id}
                type="button"
                role="radio"
                aria-checked={!patron && title === id}
                disabled={!open}
                className={progress.titleOption}
                title={open ? undefined : from}
                onClick={() => pickTitle(id)}
                data-nav
              >
                {!open && <Lock size={12} aria-hidden="true" />} {open || !from.includes('???') ? titleLabel(id) : t('career.achievements.secret')}
              </button>
            );
          })}
          <PatronTitle thanks={thanks} on={patron} />
        </div>
      </section>

      <PetShelf thanks={thanks} />
      <SupporterMarks thanks={thanks} />

      <section className={progress.shelfPanel} aria-labelledby="locker-sleeves">
        <h3 id="locker-sleeves">{t('career.look.sleeves')}</h3>
        {career && <p className={progress.panelNote}>{t('career.look.sleevesNote')}</p>}
        <div className={progress.options}>
          {career && Object.keys(CAREER_SLEEVES).map((id) => {
            const cosmetic = { kind: 'sleeve', id };
            const open = hasUnlock(unlocks, 'sleeve', id);
            return (
              <span key={id} className={[progress.item, open ? '' : progress.itemLocked].join(' ')} title={open ? undefined : how(cosmetic)}>
                <span className={progress.sleeve}><CardFace card={{ name: '' }} hidden sleeve={CAREER_SLEEVES[id]} size="normal" /></span>
                <small>{open ? cosmeticName(t, cosmetic) : <><Lock size={11} aria-hidden="true" /> {how(cosmetic)}</>}</small>
              </span>
            );
          })}
          <SupporterSleeveItem thanks={thanks} />
        </div>
      </section>

      <section className={progress.shelfPanel} aria-labelledby="locker-mats">
        <h3 id="locker-mats">{t('career.look.playmats')}</h3>
        <div className={progress.options} role="radiogroup" aria-labelledby="locker-mats">
          {career && CAREER_PLAYMATS.map((id) => {
            const cosmetic = { kind: 'playmat', id };
            const open = hasUnlock(unlocks, 'playmat', id);
            const swatch = MAT_CLOTHS.find((item) => item.id === id);
            return (
              <LockerItem key={id} on={cloth === id} open={open} label={open ? cosmeticName(t, cosmetic) : how(cosmetic)} onPick={() => useSettings.getState().update({ matCloth: id })}>
                <span className={progress.cloth} style={swatch ? clothSwatch(swatch) : undefined}><Art kind="playmat" id={id} size={56} className={progress.clothPrint} /></span>
              </LockerItem>
            );
          })}
          <PatronPlaymat thanks={thanks} />
        </div>
      </section>
    </div>
  );
}

function LockerItem({ on, open, label, onPick, children }: { on: boolean; open: boolean; label: string; onPick(): void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={on}
      disabled={!open}
      className={[progress.item, open ? '' : progress.itemLocked].join(' ')}
      onClick={onPick}
      data-nav
    >
      {children}
      <small>{!open && <Lock size={11} aria-hidden="true" />} {label}</small>
    </button>
  );
}

/** Settings, Cosmetics: the locker outside Career, with the Career track when the player has one. */
export default function CosmeticsSettings() {
  const t = useT();
  const career = useServerInfo().data?.career === true;
  const optedIn = useSettings((settings) => settings.settings.careerOptIn);
  const levels = useCareerLevels(career && optedIn);
  const achievements = useCareerAchievements(career && optedIn);
  if (levels.isLoading) return <p className={progress.panelNote}>{t('career.look.loading')}</p>;
  return (
    <div className={progress.settingsLocker}>
      <p className={progress.panelNote}>{t(career ? 'career.look.everywhere' : 'career.look.everywhereNoCareer')}</p>
      <CosmeticsLocker track={levels.data} achievements={achievements.data ?? []} career={career} />
    </div>
  );
}
