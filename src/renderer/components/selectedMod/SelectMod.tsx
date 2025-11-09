import { useCallback, useEffect, useState } from 'react';
import { List, Button, Typography } from 'antd';
import { NewMod } from './NewMod';
import { FullSizeDialogLayout } from '../FullSizeDialogLayout';
import { ErrorAlert } from '../ErrorAlert';
import { useMods } from '../../hooks/useMods';
import { useSelectedMod } from '../../hooks/useSelectedMod';
import { FullSizeLoader } from '../FullSizeLoader';
import { EditableMod } from '../../../common/invokables/mods';

export function SelectMod() {
  const { loading, error: modsError, data, refresh } = useMods();
  const { persisting, persistingError, update } = useSelectedMod();
  const editableMods = data?.editable ?? [];
  const [newMod, setNewMod] = useState(false);
  const onNewModClick = useCallback(() => setNewMod(true), [setNewMod]);
  const onNewModCancel = useCallback(() => setNewMod(false), [setNewMod]);
  const onModClick = useCallback(
    async (mod: EditableMod) => update(mod),
    [update],
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (newMod) {
    return <NewMod onCancel={onNewModCancel} />;
  }
  if (persisting || loading) {
    return <FullSizeLoader />;
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
      <ErrorAlert error={modsError || persistingError} />
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
