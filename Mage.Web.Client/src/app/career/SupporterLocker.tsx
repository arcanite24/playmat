import { Heart, Lock } from 'lucide-react';
import { Link } from 'react-router-dom';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/support';
import { clothSwatch, MAT_CLOTHS } from '../match/playmats';
import { PETS } from '../pets/pets';
import { Avatar } from '../social/Avatar';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import type { ThankYou, ThankYouKind } from '../support/supportData';
import { THANK_YOU } from '../support/supportModel';
import { CardFace } from '../ui/CardFace';
import { SUPPORTER_SLEEVE } from '../ui/sleeveArt';
import progress from './Progress.module.css';

registerMessages(messages);

/** A thank-you piece the player doesn't have: drawn dimmed with a lock; it opens the support page. */
export function SupportLockedItem({ name, className, children }: { name: string; className: string; children: React.ReactNode }) {
  const t = useT();
  const label = t('support.locker.locked', { name });
  return (
    <Link to="/support" className={[className, progress.itemLocked, progress.supportLocked].join(' ')} aria-label={label} title={label} data-nav>
      {children}
    </Link>
  );
}

/** "Patron" among the titles: the supporter's title, worn instead of a Career one. */
export function PatronTitle({ thanks, on }: { thanks: ThankYou; on: boolean }) {
  const t = useT();
  const name = t('support.locker.patron');
  if (!thanks.shows('title')) return null;
  if (!thanks.has('title')) {
    return (
      <SupportLockedItem name={name} className={progress.titleOption}>
        <Lock size={12} aria-hidden="true" /> {name}
      </SupportLockedItem>
    );
  }
  return (
    <button type="button" role="radio" aria-checked={on} className={progress.titleOption} onClick={() => thanks.choose({ title: true })} data-nav>
      <Heart size={12} aria-hidden="true" fill="currentColor" /> {name}
    </button>
  );
}

/** The thank-you item in a locker shelf: picked like the others once owned, locked otherwise. */
function ThankYouItem({ thanks, kind, item = THANK_YOU[kind], name, on, onPick, children }: {
  thanks: ThankYou;
  kind: ThankYouKind;
  item?: string;
  name: string;
  on: boolean;
  onPick(): void;
  children: React.ReactNode;
}) {
  const t = useT();
  const owned = thanks.hasItem(kind, item);
  if (!owned && !thanks.shows(kind)) return null;
  if (!owned) {
    return (
      <SupportLockedItem name={name} className={progress.item}>
        {children}
        <small><Lock size={11} aria-hidden="true" /> {t('support.locker.only')}</small>
      </SupportLockedItem>
    );
  }
  return (
    <button type="button" role="radio" aria-checked={on} className={progress.item} onClick={onPick} data-nav>
      {children}
      <small>{name}</small>
    </button>
  );
}

/** A pet's portrait for the shelf: its still, which the support page shows too. */
function PetPortrait({ still }: { still: string }) {
  return <span className={progress.pet}><img src={still} width={102} height={113} alt="" loading="lazy" /></span>;
}

/** The pets: none, or one that sits by the player's side of the battlefield in every game. */
export function PetShelf({ thanks }: { thanks: ThankYou }) {
  const t = useT();
  const pets = Object.entries(PETS).filter(([id]) => thanks.shows('pet') || thanks.hasItem('pet', id));
  if (pets.length === 0) return null;
  // a supporter wears the pet they own (one for now: the thank-you set's corgi)
  const wearing = (id: string) => thanks.wear.pet && thanks.hasItem('pet', id);
  const none = !pets.some(([id]) => wearing(id));
  return (
    <section className={progress.shelfPanel} aria-labelledby="locker-pets">
      <h3 id="locker-pets">{t('support.locker.pets')}</h3>
      <p className={progress.panelNote}>{t('support.locker.petsNote')}</p>
      <div className={progress.options} role="radiogroup" aria-labelledby="locker-pets">
        <button type="button" role="radio" aria-checked={none} className={progress.item} onClick={() => thanks.choose({ pet: false })} data-nav>
          <span className={progress.noPet} aria-hidden="true" />
          <small>{t('support.locker.noPet')}</small>
        </button>
        {pets.map(([id, pet]) => (
          <ThankYouItem key={id} thanks={thanks} kind="pet" item={id} name={t(pet.name)} on={wearing(id)} onPick={() => thanks.choose({ pet: true })}>
            <PetPortrait still={pet.still} />
          </ThankYouItem>
        ))}
      </div>
    </section>
  );
}

/** The ring round the avatar and the heart by the name: each worn or not. */
export function SupporterMarks({ thanks }: { thanks: ThankYou }) {
  const t = useT();
  const user = useSession((session) => session.userName);
  if (!thanks.shows('frame') && !thanks.shows('flair')) return null;
  const marks = [
    { kind: 'frame' as const, name: t('support.locker.ring'), preview: <Avatar name={user} size="lg" ring /> },
    { kind: 'flair' as const, name: t('support.locker.heart'), preview: <Heart className={progress.heart} size={32} fill="currentColor" aria-hidden="true" /> },
  ];
  return (
    <section className={progress.shelfPanel} aria-labelledby="locker-marks">
      <h3 id="locker-marks">{t('support.locker.marks')}</h3>
      <p className={progress.panelNote}>{t('support.locker.marksNote')}</p>
      <div className={progress.options}>
        {marks.map(({ kind, name, preview }) => {
          if (!thanks.shows(kind)) return null;
          if (!thanks.has(kind)) {
            return (
              <SupportLockedItem key={kind} name={name} className={progress.item}>
                <span className={progress.mark}>{preview}</span>
                <small><Lock size={11} aria-hidden="true" /> {t('support.locker.only')}</small>
              </SupportLockedItem>
            );
          }
          const on = thanks.wear[kind];
          return (
            <button key={kind} type="button" role="switch" aria-checked={on} aria-label={name} className={progress.item} onClick={() => thanks.choose({ [kind]: !on })} data-nav>
              <span className={progress.mark}>{preview}</span>
              <small>{name}</small>
            </button>
          );
        })}
      </div>
    </section>
  );
}

/** The supporter sleeve beside the Career ones: chosen per deck, like them. */
export function SupporterSleeveItem({ thanks }: { thanks: ThankYou }) {
  const t = useT();
  const name = t('support.sleeve');
  const face = <span className={progress.sleeve}><CardFace card={{ name: '' }} hidden sleeve={SUPPORTER_SLEEVE} size="normal" /></span>;
  if (!thanks.shows('sleeve')) return null;
  if (!thanks.has('sleeve')) {
    return (
      <SupportLockedItem name={name} className={progress.item}>
        {face}
        <small><Lock size={11} aria-hidden="true" /> {t('support.locker.only')}</small>
      </SupportLockedItem>
    );
  }
  return <span className={progress.item}>{face}<small>{name}</small></span>;
}

/** The Patron cloth beside the Career playmats. */
export function PatronPlaymat({ thanks }: { thanks: ThankYou }) {
  const t = useT();
  const cloth = useSettings((settings) => settings.settings.matCloth);
  const swatch = MAT_CLOTHS.find((item) => item.id === THANK_YOU.playmat);
  if (!swatch) return null;
  return (
    <ThankYouItem
      thanks={thanks}
      kind="playmat"
      name={t('support.locker.patronMat')}
      on={cloth === swatch.id}
      onPick={() => useSettings.getState().update({ matCloth: swatch.id })}
    >
      <span className={progress.cloth} style={clothSwatch(swatch)} />
    </ThankYouItem>
  );
}
