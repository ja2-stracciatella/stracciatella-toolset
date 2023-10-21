import { useCallback } from 'react';
import { List, Button, Alert } from 'antd';
import './OpenMod.css';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { EditableMod, setSelectedMod } from '../state/mods';

const { Item } = List;

export function OpenMod() {
  const dispatch = useAppDispatch();
  const error = useAppSelector((s) => s.mods.error);
  const editableMods = useAppSelector((s) => s.mods.mods?.editable) ?? [];
  const onModClick = useCallback(
    async (mod: EditableMod) => {
      dispatch(setSelectedMod(mod));
    },
    [dispatch],
  );

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  return (
    <div className="open-root">
      <h2>
        Select mod to edit
        <Button>Add new mod</Button>
      </h2>
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
