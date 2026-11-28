import { act, fireEvent, render } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { PermanentView } from '../../protocol/generated/views';
import { PermanentStack } from './PermanentStack';
import { fitRow, type PermanentGroup } from './boardModel';
import { useMatchUi } from './matchUi';

it.each([0, 1, 6, 12])('keeps the host close to %i attachments and within its row', (count) => {
  const host: PermanentView = { id: 'host', name: 'Kor Spiritdancer', cardTypes: ['CREATURE'] };
  const attachments = Array.from({ length: count }, (_, i): PermanentView => ({ id: `aura${i}`, name: `Aura ${i}`, cardTypes: ['ENCHANTMENT'] }));
  const group: PermanentGroup = { key: 'host', lead: host, members: [host], attachments };
  const fit = fitRow([group], 1126, 202, 150, 58);
  const onClick = vi.fn();
  const { container } = render(<PermanentStack group={group} width={fit.cardWidth} sleeve="default" clickable={new Set(attachments.map((card) => card.id!))} selected={new Set()} attacking={new Set()} blocking={new Set()} forward={1} onClick={onClick} />);
  const slot = container.firstElementChild as HTMLElement;
  const card = container.querySelector<HTMLElement>('[data-object-id="host"]')!;
  expect(parseFloat(card.style.top)).toBeCloseTo(count > 0 ? fit.cardWidth * 0.18 : 0);
  expect(parseFloat(slot.style.height)).toBeLessThanOrEqual(202.5);
  expect(parseFloat(card.style.top) + parseFloat(card.style.height)).toBeCloseTo(parseFloat(slot.style.height));
  for (const attachment of attachments) {
    const aura = container.querySelector<HTMLElement>(`[data-object-id="${attachment.id}"]`)!;
    expect(aura.style.top).toBe('0px');
    fireEvent.click(aura);
    expect(onClick).toHaveBeenLastCalledWith(attachment.id);
  }
});

it('marks the permanent a hovered stack ability or an open question comes from (#227)', () => {
  const hydra = (id: string): PermanentView => ({ id, name: 'Guiding Hydra', cardTypes: ['CREATURE'] });
  const groups: PermanentGroup[] = ['hydra-1', 'hydra-2'].map((id) => ({ key: id, lead: hydra(id), members: [hydra(id)], attachments: [] }));
  const { container } = render(<>{groups.map((group) => <PermanentStack key={group.key} group={group} width={120} sleeve="default" clickable={new Set()} selected={new Set()} attacking={new Set()} blocking={new Set()} forward={1} onClick={() => undefined} />)}</>);
  const marked = () => [...container.querySelectorAll('[data-object-id]')].filter((card) => card.getAttribute('aria-label')?.includes('source of the ability')).map((card) => card.getAttribute('data-object-id'));
  expect(marked()).toEqual([]);
  act(() => useMatchUi.setState({ promptSource: 'hydra-2' }));
  expect(marked()).toEqual(['hydra-2']);
  // hovering the other one's ability on the stack marks that one instead
  act(() => useMatchUi.getState().setSourceHover('hydra-1'));
  expect(marked()).toEqual(['hydra-1']);
  act(() => useMatchUi.setState({ sourceHover: null, promptSource: null }));
  expect(marked()).toEqual([]);
});

it('drops the block line when its card leaves mid-drag or loses the pointer', () => {
  const proto = HTMLElement.prototype as unknown as Record<string, unknown>;
  proto.setPointerCapture ??= () => undefined;
  proto.hasPointerCapture ??= () => false;
  proto.releasePointerCapture ??= () => undefined;
  const doc = document as unknown as Record<string, unknown>;
  doc.elementsFromPoint ??= () => [];
  const bear: PermanentView = { id: 'bear', name: 'Grizzly Bears', cardTypes: ['CREATURE'] };
  const group: PermanentGroup = { key: 'bear', lead: bear, members: [bear], attachments: [] };
  const stack = () => <PermanentStack group={group} width={120} sleeve="default" clickable={new Set(['bear'])} selected={new Set()} attacking={new Set()} blocking={new Set()} forward={1} onClick={() => undefined} onBlockDrop={() => true} />;
  const drag = (card: HTMLElement) => {
    fireEvent.pointerDown(card, { button: 0, pointerId: 1, clientX: 10, clientY: 10 });
    fireEvent.pointerMove(card, { pointerId: 1, clientX: 200, clientY: 200 });
  };

  const first = render(stack());
  drag(first.container.querySelector<HTMLElement>('[data-object-id="bear"]')!);
  expect(useMatchUi.getState().blockDrag?.blockerId).toBe('bear');
  first.unmount();
  expect(useMatchUi.getState().blockDrag).toBeNull();

  const second = render(stack());
  const card = second.container.querySelector<HTMLElement>('[data-object-id="bear"]')!;
  drag(card);
  expect(useMatchUi.getState().blockDrag).not.toBeNull();
  act(() => {
    card.dispatchEvent(new Event('lostpointercapture', { bubbles: true }));
  });
  expect(useMatchUi.getState().blockDrag).toBeNull();
  second.unmount();
});
