import { useCallback, useId, useMemo, useState, useSyncExternalStore } from 'react';
import { imageResolver } from '../../core/images/useCardImage';
import { useT } from '../i18n';
import { CardFace } from './CardFace';
import { COMMON_TOKENS, parsePicture, pictureCard, type TokenChoices } from './tokenArt';
import styles from './TokenArtPicker.module.css';

/** The token lists' names, or null while they load. */
function useTokenNames(): string[] | null {
  const resolver = imageResolver();
  const subscribe = useCallback((listener: () => void) => resolver.subscribeKey(null, listener), [resolver]);
  const loaded = useSyncExternalStore(subscribe, () => resolver.linksLoaded());
  return useMemo(() => (loaded ? resolver.tokenPictures() : null), [resolver, loaded]);
}

/** One picture as the picker's thumbnail. */
function Thumb({ picture }: { picture: string }) {
  const card = useMemo(() => pictureCard(picture), [picture]);
  return card ? <CardFace card={card} size="small" className={styles.face} /> : null;
}

/**
 * Token art (ui/tokenArt.ts): pick a token by name, then which of its pictures to show. Used for the account (in the
 * settings) and for one deck (in the deck builder), where `inherited` are the account's choices the deck keeps
 * unless it picks its own.
 */
export function TokenArtPicker({ choices, onChoose, inherited }: {
  choices: TokenChoices;
  onChoose(name: string, picture: string | null): void;
  inherited?: TokenChoices;
}) {
  const t = useT();
  const listId = useId();
  const names = useTokenNames();
  const [query, setQuery] = useState('');
  const known = useMemo(() => new Set(names ?? []), [names]);
  const name = known.has(query.trim()) ? query.trim() : null;
  const pictures = useMemo(() => (name ? imageResolver().tokenPictures(name) ?? [] : []), [name]);
  const common = useMemo(() => COMMON_TOKENS.filter((token) => known.has(token)), [known]);
  const chosen = Object.entries(choices).sort(([a], [b]) => a.localeCompare(b));
  const current = name ? choices[name] ?? null : null;
  const fallback = name ? inherited?.[name] ?? null : null;

  return (
    <div className={styles.picker}>
      {chosen.length > 0 && (
        <ul className={styles.chosen} aria-label={t('tokenArt.chosen')}>
          {chosen.map(([token, picture]) => (
            <li key={token} className={styles.choice}>
              <button type="button" className={styles.choiceButton} onClick={() => setQuery(token)} aria-label={t('tokenArt.change', { token })}>
                <Thumb picture={picture} />
                <span className={styles.choiceName}>{token}</span>
                <span className={styles.set}>{parsePicture(picture)?.set}</span>
              </button>
              <button type="button" className={styles.reset} onClick={() => onChoose(token, null)}>{t('tokenArt.reset')}</button>
            </li>
          ))}
        </ul>
      )}
      <label className={styles.search}>
        <span>{t('tokenArt.token')}</span>
        <input
          list={listId}
          value={query}
          placeholder={names ? t('tokenArt.search') : t('tokenArt.loading')}
          disabled={!names}
          onChange={(event) => setQuery(event.target.value)}
        />
        <datalist id={listId}>{(names ?? []).map((token) => <option key={token} value={token} />)}</datalist>
      </label>
      {!name && common.length > 0 && (
        <div className={styles.common} aria-label={t('tokenArt.common')}>
          {common.map((token) => (
            <button key={token} type="button" className={styles.chip} aria-pressed={!!choices[token]} onClick={() => setQuery(token)}>{token}</button>
          ))}
        </div>
      )}
      {name && (
        <div className={styles.grid} role="group" aria-label={t('tokenArt.pictures', { token: name })}>
          <button type="button" className={styles.option} aria-pressed={!current} onClick={() => onChoose(name, null)}>
            <span className={styles.plain}>{fallback ? t('tokenArt.account') : t('tokenArt.default')}</span>
            {fallback && <Thumb picture={fallback} />}
          </button>
          {pictures.map((picture) => (
            <button key={picture} type="button" className={styles.option} aria-pressed={current === picture} onClick={() => onChoose(name, picture)} aria-label={picture}>
              <Thumb picture={picture} />
              <span className={styles.set}>{parsePicture(picture)?.set}{parsePicture(picture)?.imageNumber ? ` · ${parsePicture(picture)?.imageNumber}` : ''}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
