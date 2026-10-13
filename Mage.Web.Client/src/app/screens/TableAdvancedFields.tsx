import { ChevronDown } from 'lucide-react';
import { useId, useState, type ReactNode } from 'react';
import {
  ATTACK_OPTIONS, BUFFER_TIMES, describeAdvanced, MULLIGAN_TYPES, parseSeed, RANGES, SKILL_LEVELS, TIME_LIMITS, type TableAdvanced,
} from '../../core/game/tableSetup';
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
  const id = useId();
  const invalid = value.trim() !== '' && parseSeed(value) === null;
  return (
    <div className={styles.field}>
      <label htmlFor={id}>Seed</label>
      <input
        id={id}
        inputMode="numeric"
        placeholder="Random"
        value={value}
        aria-invalid={invalid}
        onChange={(event) => onChange(event.target.value)}
      />
      <p className={styles.hint}>{invalid ? 'Up to 15 digits.' : 'The same seed deals the same cards, to replay a game.'}</p>
    </div>
  );
}

const LIVES = [{ value: 0, label: 'The format’s own' }, ...[10, 20, 25, 30, 40, 50, 60].map((life) => ({ value: life, label: `${life}` }))];
const HANDS = [{ value: 0, label: 'Seven' }, ...[4, 5, 6, 8, 9, 10].map((size) => ({ value: size, label: `${size}` }))];
const FREE_MULLIGANS = [0, 1, 2, 3].map((count) => ({ value: count, label: count === 0 ? 'None' : `${count}` }));
const POWER_LEVELS = [{ value: 0, label: 'Any' }, ...Array.from({ length: 10 }, (_, index) => ({ value: index + 1, label: `Up to ${index + 1}` }))];
const RATINGS = [0, 1000, 1200, 1400, 1600, 1800].map((rating) => ({ value: rating, label: rating === 0 ? 'Anyone' : `${rating}+` }));

/** The host dialog's "Advanced" section: closed by default, with a one-line summary of what was changed. */
export function TableAdvancedFields({ value, onChange, multiplayer, commander }: {
  value: TableAdvanced;
  onChange(patch: Partial<TableAdvanced>): void;
  multiplayer: boolean;
  commander: boolean;
}) {
  const [open, setOpen] = useState(false);
  const changed = describeAdvanced(value);
  const bodyId = useId();
  return (
    <section className={styles.advanced}>
      <button type="button" className={styles.toggle} aria-expanded={open} aria-controls={bodyId} onClick={() => setOpen(!open)}>
        <ChevronDown size={16} className={open ? styles.chevronOpen : styles.chevron} aria-hidden="true" />
        Advanced
        <span className={styles.summary}>{changed.length > 0 ? changed.join(' · ') : 'Standard rules'}</span>
      </button>
      {open && (
        <div id={bodyId} className={styles.grid}>
          <Select label="Mulligan" value={value.mulliganType} options={MULLIGAN_TYPES} onChange={(mulliganType) => onChange({ mulliganType })} />
          <Select label="Free mulligans" value={value.freeMulligans} options={FREE_MULLIGANS} onChange={(freeMulligans) => onChange({ freeMulligans })} />
          <Select label="Starting life" value={value.startingLife ?? 0} options={LIVES} onChange={(life) => onChange({ startingLife: life === 0 ? null : life })} />
          <Select label="Opening hand" value={value.handSize ?? 0} options={HANDS} onChange={(size) => onChange({ handSize: size === 0 ? null : size })} />
          <Select label="Match clock" value={value.matchTimeLimit} options={TIME_LIMITS} onChange={(matchTimeLimit) => onChange({ matchTimeLimit })} hint="Each player's total time for the match." />
          <Select label="Time per decision" value={value.matchBufferTime} options={BUFFER_TIMES} onChange={(matchBufferTime) => onChange({ matchBufferTime })} hint="Spent before the match clock runs." />
          <Select label="Players' skill" value={value.skillLevel} options={SKILL_LEVELS} onChange={(skillLevel) => onChange({ skillLevel })} />
          <SeedField value={value.seed} onChange={(seed) => onChange({ seed })} />
          <Select label="Minimum rating" value={value.minimumRating} options={RATINGS} onChange={(minimumRating) => onChange({ minimumRating })} />
          {commander && (
            <Select label="Deck power level" value={value.edhPowerLevel} options={POWER_LEVELS} onChange={(edhPowerLevel) => onChange({ edhPowerLevel })} hint="Decks above it can't join." />
          )}
          {multiplayer && (
            <>
              <Select
                label="Who can be attacked"
                value={value.attackOption}
                options={ATTACK_OPTIONS}
                onChange={(attackOption) => onChange({ attackOption })}
                hint={ATTACK_OPTIONS.find((option) => option.value === value.attackOption)?.detail}
              />
              <Select
                label="Range of influence"
                value={value.range}
                options={RANGES}
                onChange={(range) => onChange({ range })}
                hint={RANGES.find((option) => option.value === value.range)?.detail}
              />
            </>
          )}
          <div className={styles.checks}>
            <Check label="Rated" checked={value.rated} onChange={(rated) => onChange({ rated })} />
            <Check label="Planechase" checked={value.planeChase} onChange={(planeChase) => onChange({ planeChase })} />
            <Check label="Spectators allowed" checked={value.spectatorsAllowed} onChange={(spectatorsAllowed) => onChange({ spectatorsAllowed })} />
            <Check label="Rollback requests" checked={value.rollbackTurnsAllowed} onChange={(rollbackTurnsAllowed) => onChange({ rollbackTurnsAllowed })} />
          </div>
        </div>
      )}
    </section>
  );
}
