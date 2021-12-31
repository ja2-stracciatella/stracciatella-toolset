import { useCallback, useState } from "react";
import { ListGroup, Button } from "react-bootstrap";

import { FullSizeLoader } from "../components/FullSizeLoader";
import { EditableMod, useMods } from "../state/mods";
import "./Open.css";

export function Open() {
  const { selectMod, editableMods } = useMods();
  const [isSelecting, setIsSelecting] = useState(false);
  const onModClick = useCallback(
    async (mod: EditableMod) => {
      setIsSelecting(true);
      await selectMod(mod);
      setIsSelecting(false);
    },
    [selectMod]
  );

  if (!editableMods || isSelecting) {
    return <FullSizeLoader />;
  }
  return (
    <div className="open-root">
      <h2>
        Select mod to edit
        <Button variant="primary">Add new mod</Button>
      </h2>
      <ListGroup>
        {editableMods.map((mod) => (
          <ListGroup.Item key={mod.id} onClick={() => onModClick(mod)}>
            {mod.name} ({mod.version})
          </ListGroup.Item>
        ))}
      </ListGroup>
    </div>
  );
}
