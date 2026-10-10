import { Flex, Select, Typography } from 'antd';
import { useCallback, useMemo } from 'react';
import { DetailedPlacement } from '../../../lib/mapViewer';
import { ProfileInfo } from './soldierData';
import { sendSoldierCommand } from './soldierEdit';

export interface SoldierProfileSelectProps {
  detailed: DetailedPlacement;
  profiles: ProfileInfo[];
}

/**
 * The named character a detailed placement is (`MERC_PROFILEMODE`). The profile decides the
 * attributes, the items and the colours.
 */
export function SoldierProfileSelect({
  detailed,
  profiles,
}: SoldierProfileSelectProps) {
  const options = useMemo(() => {
    const options = profiles.map((profile) => ({
      value: profile.id,
      label: `${profile.id} ${profile.name}`,
    }));
    const current = detailed.profile;
    if (current !== null && !options.some((o) => o.value === current)) {
      options.push({ value: current, label: `Profile ${current}` });
    }
    return options;
  }, [detailed.profile, profiles]);
  const onChange = useCallback(
    (profile: number) =>
      sendSoldierCommand({ kind: 'detailed', detailed: true, profile }),
    [],
  );
  const style = useMemo(() => ({ width: '100%' }) as const, []);
  return (
    <Flex vertical gap={2}>
      <Typography.Text type="secondary">Profile</Typography.Text>
      <Select<number>
        size="small"
        showSearch
        optionFilterProp="label"
        style={style}
        options={options}
        value={detailed.profile ?? undefined}
        onChange={onChange}
        aria-label="profile"
      />
      <Typography.Text type="secondary">
        The profile decides the attributes, the items and the colours.
      </Typography.Text>
    </Flex>
  );
}
