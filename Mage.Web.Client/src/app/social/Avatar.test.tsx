import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { flagChoices, flagLabel, SIGILS } from '../../core/social/identity';
import { Avatar, Flag } from './Avatar';

describe('player identity symbols', () => {
  it.each(SIGILS)('draws the saved $label sigil as an SVG', ({ id }) => {
    const { container } = render(<Avatar name="Player" avatarId={id} />);
    const svg = container.querySelector('svg');
    expect(svg?.getAttribute('aria-hidden')).toBe('true');
    expect(svg?.querySelector('path')?.getAttribute('d')).toBeTruthy();
    expect(container.textContent).toBe('');
  });

  it.each([undefined, 10, 11, 51])('keeps the initial for avatar id %s', (avatarId) => {
    const { container } = render(<Avatar name="Player" avatarId={avatarId} />);
    expect(container.textContent).toBe('P');
    expect(container.querySelector('svg')).toBeNull();
  });

  it('keeps flag names accessible while displaying compact text', () => {
    render(<Flag flagName="de.png" />);
    expect(screen.getByRole('img', { name: flagLabel('de') }).textContent).toBe('DE');
  });

  it('hides the default flag unless explicitly requested', () => {
    const { container, rerender } = render(<Flag />);
    expect(container.childElementCount).toBe(0);
    rerender(<Flag always />);
    expect(screen.getByRole('img', { name: 'Anywhere' }).querySelector('svg')).not.toBeNull();
  });

  it('uses a globe for unsupported desktop flags', () => {
    render(<Flag flagName="genderfluid.png" />);
    expect(screen.getByRole('img').querySelector('svg')).not.toBeNull();
  });

  it('renders every flag choice without emoji text', () => {
    const { container } = render(<>{flagChoices().map(({ code }) => <Flag key={code} flagName={code} always />)}</>);
    expect(container.textContent).not.toMatch(/[\p{Extended_Pictographic}\p{Regional_Indicator}]/u);
    expect(container.querySelectorAll('[role="img"]')).toHaveLength(flagChoices().length);
  });
});
