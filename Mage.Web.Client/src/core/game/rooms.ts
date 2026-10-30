import type { PermanentView } from '../../protocol/generated/views';

// apart from cards.ts, which loads with the first page: only the board shows a Room's doors

export interface RoomDoor {
  name: string;
  unlocked: boolean;
}

/** A Room permanent's two doors, left first, as the server tells them (null: not a face-up Room). */
export function roomDoors(permanent: Pick<PermanentView, 'roomLeftDoor' | 'roomRightDoor' | 'roomLeftUnlocked' | 'roomRightUnlocked'>): [RoomDoor, RoomDoor] | null {
  if (!permanent.roomLeftDoor || !permanent.roomRightDoor) return null;
  return [
    { name: permanent.roomLeftDoor, unlocked: !!permanent.roomLeftUnlocked },
    { name: permanent.roomRightDoor, unlocked: !!permanent.roomRightUnlocked },
  ];
}
