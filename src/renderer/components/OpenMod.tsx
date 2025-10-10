import { useCallback, useState } from 'react';
import { List, Button, Alert, Typography, Space } from 'antd';
import './OpenMod.css';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { EditableMod, setSelectedMod } from '../state/mods';
import { NewMod } from './NewMod';

const { Item } = List;

export function OpenMod() {
  const dispatch = useAppDispatch();
  const error = useAppSelector((s) => s.mods.error);
  const editableMods = useAppSelector((s) => s.mods.mods?.editable) ?? [];
  const [newMod, setNewMod] = useState(false);
  const onNewModClick = useCallback(() => setNewMod(true), [setNewMod]);
  const onModClick = useCallback(
    async (mod: EditableMod) => {
      dispatch(setSelectedMod(mod));
    },
    [dispatch],
  );

  if (newMod) {
    return <NewMod />;
  }
  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  return (
    <div className="open-mod-root">
      <Typography.Title>
        <Space>
          Select mod to edit
          <Button onClick={onNewModClick}>Create new mod</Button>
        </Space>
      </Typography.Title>
      <List>
        {editableMods.map((mod) => (
          <Item key={mod.id} onClick={() => onModClick(mod)}>
            {mod.name} ({mod.version})
          </Item>
        ))}
      </List>
    </div>
  );
}
