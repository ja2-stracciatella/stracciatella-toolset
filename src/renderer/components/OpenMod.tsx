import { useCallback, useMemo, useState } from 'react';
import { List, Button, Alert, Typography, Space } from 'antd';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { EditableMod, setSelectedMod } from '../state/mods';
import { NewMod } from './NewMod';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { ErrorAlert } from './ErrorAlert';

export function OpenMod() {
  const dispatch = useAppDispatch();
  const error = useAppSelector((s) => s.mods.error);
  const editableMods = useAppSelector((s) => s.mods.mods?.editable) ?? [];
  const [newMod, setNewMod] = useState(false);
  const onNewModClick = useCallback(() => setNewMod(true), [setNewMod]);
  const onNewModCancel = useCallback(() => setNewMod(false), [setNewMod]);
  const onModClick = useCallback(
    async (mod: EditableMod) => {
      dispatch(setSelectedMod(mod));
    },
    [dispatch],
  );

  if (newMod) {
    return <NewMod onCancel={onNewModCancel} />;
  }
  return (
    <FullSizeDialogLayout>
      <Typography.Title>Select mod to edit</Typography.Title>
      <Typography.Paragraph>
        Select the mod you want to edit or create a new mod.
      </Typography.Paragraph>
      <Typography.Paragraph>
        <Button onClick={onNewModClick}>Create new mod</Button>
      </Typography.Paragraph>
      <Typography.Title>Editable Mods</Typography.Title>
      <ErrorAlert error={error} />
      <List
        dataSource={editableMods}
        renderItem={(mod) => (
          <List.Item
            onClick={() => onModClick(mod)}
            extra={<Typography.Text>{mod.version}</Typography.Text>}
          >
            <List.Item.Meta
              title={
                <Typography.Link>{`${mod.name} (${mod.id})`}</Typography.Link>
              }
              description={mod.description}
            />
          </List.Item>
        )}
      />
    </FullSizeDialogLayout>
  );
}
