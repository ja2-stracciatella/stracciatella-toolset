import { useCallback, useState } from 'react';
import { List, Button, Alert } from 'antd';

import { FullSizeLoader } from './FullSizeLoader';
import { EditableMod, useMods } from '../state/mods';
import './OpenMod.css';

const { Item } = List;

export function OpenMod() {
  const { error, selectMod, editableMods } = useMods();
  const [isSelecting, setIsSelecting] = useState(false);
  const onModClick = useCallback(
    async (mod: EditableMod) => {
      setIsSelecting(true);
      await selectMod(mod);
    },
    [selectMod]
  );

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  if (!editableMods || isSelecting) {
    return <FullSizeLoader />;
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
