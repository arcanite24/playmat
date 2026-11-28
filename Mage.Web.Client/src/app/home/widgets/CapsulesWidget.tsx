import { useT } from '../../i18n';
import { useCareerState } from '../../career/careerData';
import type { WidgetProps } from '../widgets';
import { Big, Note, Tile } from './Tile';

/** Capsule tokens to spend. */
export default function CapsulesWidget({ editing }: WidgetProps) {
  const t = useT();
  const tokens = useCareerState().data?.profile?.capsuleTokens ?? 0;
  return (
    <Tile title={t('home.widget.capsules')} to="/career/capsules" editing={editing}>
      <Big>{tokens}</Big>
      <Note>{t('home.widget.capsules.tokens', { count: tokens })}</Note>
    </Tile>
  );
}
