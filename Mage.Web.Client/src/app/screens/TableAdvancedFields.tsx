import { ChevronDown } from 'lucide-react';
import { useId, useMemo, useState, type ReactNode } from 'react';
import {
  ATTACK_OPTIONS, BUFFER_TIMES, DEFAULT_ADVANCED, MULLIGAN_TYPES, parseSeed, RANGES, SKILL_LEVELS, TIME_LIMITS, type TableAdvanced,
} from '../../core/game/tableSetup';
import { useT, type MessageKey, type Translate } from '../i18n';
import styles from './TableAdvancedFields.module.css';

function Select<T extends string | number>({ label, value, options, onChange, hint }: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange(value: T): void;
  hint?: ReactNode;
}) {
  const id = useId();
  return (
    <div className={styles.field}>
      <label htmlFor={id}>{label}</label>
      <select
        id={id}
        value={String(value)}
        onChange={(event) => {
          const picked = options.find((option) => String(option.value) === event.target.value);
          if (picked) onChange(picked.value);
        }}
      >
        {options.map((option) => <option key={String(option.value)} value={String(option.value)}>{option.label}</option>)}
      </select>
      {hint && <p className={styles.hint}>{hint}</p>}
    </div>
  );
}

function Check({ label, checked, onChange }: { label: string; checked: boolean; onChange(checked: boolean): void }) {
  return (
    <label className={styles.check}>
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      {label}
    </label>
  );
}

function SeedField({ value, onChange }: { value: string; onChange(value: string): void }) {
  const t = useT();
  const id = useId();
  const invalid = value.trim() !== '' && parseSeed(value) === null;
  return (
    <div className={styles.field}>
      <label htmlFor={id}>{t('tables.adv.seed')}</label>
      <input
        id={id}
        inputMode="numeric"
        placeholder={t('tables.adv.seed.random')}
        value={value}
        aria-invalid={invalid}
        onChange={(event) => onChange(event.target.value)}
      />
      <p className={styles.hint}>{t(invalid ? 'tables.adv.seed.invalid' : 'tables.adv.seed.hint')}</p>
    </div>
  );
}

/** The option lists, labelled in the player's language (tableSetup's labels are English). */
function useOptions(t: Translate) {
  return useMemo(() => ({
    lives: [{ value: 0, label: t('tables.adv.formatOwn') }, ...[10, 20, 25, 30, 40, 50, 60].map((life) => ({ value: life, label: `${life}` }))],
    hands: [{ value: 0, label: t('tables.adv.seven') }, ...[4, 5, 6, 8, 9, 10].map((size) => ({ value: size, label: `${size}` }))],
    freeMulligans: [0, 1, 2, 3].map((count) => ({ value: count, label: count === 0 ? t('tables.adv.none') : `${count}` })),
    powerLevels: [{ value: 0, label: t('tables.adv.any') }, ...Array.from({ length: 10 }, (_, index) => ({ value: index + 1, label: t('tables.adv.upTo', { count: index + 1 }) }))],
    ratings: [0, 1000, 1200, 1400, 1600, 1800].map((rating) => ({ value: rating, label: rating === 0 ? t('tables.adv.anyone') : `${rating}+` })),
    // the mulligan rules are named after places, the same in every language; only "the format's own" is translated
    mulligans: MULLIGAN_TYPES.map((option) => (option.value === 'GAME_DEFAULT' ? { ...option, label: t('tables.adv.formatOwn') } : option)),
    timeLimits: TIME_LIMITS.map((option) => ({ ...option, label: option.value === 'NONE' ? t('tables.adv.noLimit') : t('tables.adv.minutes', { count: parseInt(option.label, 10) }) })),
    bufferTimes: BUFFER_TIMES.map((option) => (option.value === 'NONE' ? { ...option, label: t('tables.adv.none') } : option)),
    skills: SKILL_LEVELS.map((option) => ({ ...option, label: t(`tables.adv.skill.${option.value}` as MessageKey) })),
    attacks: ATTACK_OPTIONS.map((option) => ({
      ...option,
      label: t(`tables.adv.attack.${option.value}` as MessageKey),
      detail: t(`tables.adv.attack.${option.value}.detail` as MessageKey),
    })),
    ranges: RANGES.map((option) => ({
      ...option,
      label: t(`tables.adv.range.${option.value}` as MessageKey),
      detail: t(`tables.adv.range.${option.value}.detail` as MessageKey),
    })),
  }), [t]);
}

/** The settings that differ from the defaults, as short phrases (tableSetup's describeAdvanced, translated). */
function describe(t: Translate, advanced: TableAdvanced, options: ReturnType<typeof useOptions>): string[] {
  const parts: string[] = [];
  if (advanced.mulliganType !== DEFAULT_ADVANCED.mulliganType) {
    parts.push(t('tables.adv.sum.mulligan', { name: options.mulligans.find((item) => item.value === advanced.mulliganType)?.label ?? '' }));
  }
  if (parseSeed(advanced.seed) !== null) parts.push(t('tables.adv.sum.seed', { seed: advanced.seed.trim() }));
  if (advanced.freeMulligans > 0) parts.push(t('tables.adv.sum.free', { count: advanced.freeMulligans }));
  if (advanced.matchTimeLimit !== 'NONE') parts.push(options.timeLimits.find((item) => item.value === advanced.matchTimeLimit)?.label ?? '');
  if (advanced.rated) parts.push(t('tables.adv.rated'));
  if (advanced.planeChase) parts.push(t('tables.adv.planechase'));
  if (advanced.startingLife !== null) parts.push(t('tables.adv.sum.life', { count: advanced.startingLife }));
  if (advanced.edhPowerLevel > 0) parts.push(t('tables.adv.sum.power', { count: advanced.edhPowerLevel }));
  if (advanced.attackOption !== 'MULTIPLE') parts.push(t(advanced.attackOption === 'LEFT' ? 'tables.adv.sum.attackLeft' : 'tables.adv.sum.attackRight'));
  if (advanced.range !== 'ALL') parts.push(t(advanced.range === 'ONE' ? 'tables.adv.sum.rangeOne' : 'tables.adv.sum.rangeTwo'));
  if (!advanced.spectatorsAllowed) parts.push(t('tables.adv.sum.noSpectators'));
  return parts.filter(Boolean);
}

/** The host dialog's "Advanced" section: closed by default, with a one-line summary of what was changed. */
export function TableAdvancedFields({ value, onChange, multiplayer, commander }: {
  value: TableAdvanced;
  onChange(patch: Partial<TableAdvanced>): void;
  multiplayer: boolean;
  commander: boolean;
}) {
  const t = useT();
  const options = useOptions(t);
  const [open, setOpen] = useState(false);
  const changed = describe(t, value, options);
  const bodyId = useId();
  return (
    <section className={styles.advanced}>
      <button type="button" className={styles.toggle} aria-expanded={open} aria-controls={bodyId} onClick={() => setOpen(!open)}>
        <ChevronDown size={16} className={open ? styles.chevronOpen : styles.chevron} aria-hidden="true" />
        {t('tables.adv.title')}
        <span className={styles.summary}>{changed.length > 0 ? changed.join(' · ') : t('tables.adv.standard')}</span>
      </button>
      {open && (
        <div id={bodyId} className={styles.grid}>
          <Select label={t('tables.adv.mulligan')} value={value.mulliganType} options={options.mulligans} onChange={(mulliganType) => onChange({ mulliganType })} />
          <Select label={t('tables.adv.freeMulligans')} value={value.freeMulligans} options={options.freeMulligans} onChange={(freeMulligans) => onChange({ freeMulligans })} />
          <Select label={t('tables.adv.life')} value={value.startingLife ?? 0} options={options.lives} onChange={(life) => onChange({ startingLife: life === 0 ? null : life })} />
          <Select label={t('tables.adv.hand')} value={value.handSize ?? 0} options={options.hands} onChange={(size) => onChange({ handSize: size === 0 ? null : size })} />
          <Select label={t('tables.adv.clock')} value={value.matchTimeLimit} options={options.timeLimits} onChange={(matchTimeLimit) => onChange({ matchTimeLimit })} hint={t('tables.adv.clock.hint')} />
          <Select label={t('tables.adv.buffer')} value={value.matchBufferTime} options={options.bufferTimes} onChange={(matchBufferTime) => onChange({ matchBufferTime })} hint={t('tables.adv.buffer.hint')} />
          <Select label={t('tables.adv.skill')} value={value.skillLevel} options={options.skills} onChange={(skillLevel) => onChange({ skillLevel })} />
          <SeedField value={value.seed} onChange={(seed) => onChange({ seed })} />
          <Select label={t('tables.adv.rating')} value={value.minimumRating} options={options.ratings} onChange={(minimumRating) => onChange({ minimumRating })} />
          {commander && (
            <Select label={t('tables.adv.power')} value={value.edhPowerLevel} options={options.powerLevels} onChange={(edhPowerLevel) => onChange({ edhPowerLevel })} hint={t('tables.adv.power.hint')} />
          )}
          {multiplayer && (
            <>
              <Select
                label={t('tables.adv.attack')}
                value={value.attackOption}
                options={options.attacks}
                onChange={(attackOption) => onChange({ attackOption })}
                hint={options.attacks.find((option) => option.value === value.attackOption)?.detail}
              />
              <Select
                label={t('tables.adv.range')}
                value={value.range}
                options={options.ranges}
                onChange={(range) => onChange({ range })}
                hint={options.ranges.find((option) => option.value === value.range)?.detail}
              />
            </>
          )}
          <div className={styles.checks}>
            <Check label={t('tables.adv.rated')} checked={value.rated} onChange={(rated) => onChange({ rated })} />
            <Check label={t('tables.adv.planechase')} checked={value.planeChase} onChange={(planeChase) => onChange({ planeChase })} />
            <Check label={t('tables.adv.spectators')} checked={value.spectatorsAllowed} onChange={(spectatorsAllowed) => onChange({ spectatorsAllowed })} />
            <Check label={t('tables.adv.rollback')} checked={value.rollbackTurnsAllowed} onChange={(rollbackTurnsAllowed) => onChange({ rollbackTurnsAllowed })} />
          </div>
        </div>
      )}
    </section>
  );
}
