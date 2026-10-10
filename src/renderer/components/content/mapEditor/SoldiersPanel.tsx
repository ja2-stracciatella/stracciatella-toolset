import { Alert, Flex, Segmented, Typography } from 'antd';
import { Dispatch, useCallback, useMemo } from 'react';
import {
  EditorSoldierEvent,
  SOLDIER_TEAMS,
  SoldierPaletteEvent,
  SoldierTeam,
  SoldierTool,
} from '../../../lib/mapViewer';
import { BRUSHES, BrushControls } from './BrushControls';
import { EraseControl } from './EraseControl';
import { SoldierEditor } from './SoldierEditor';
import { SoldierModeAction, SoldierModeState } from './soldierMode';
import type { BrushKind } from './terrainMode';
import { ToolOption, ToolSelect } from './ToolSelect';

export const TEAM_LABELS: Record<SoldierTeam, string> = {
  enemy: 'Enemies',
  creature: 'Creatures',
  rebel: 'Rebels',
  civilian: 'Civilians',
};

const TOOLS: ToolOption<SoldierTool>[] = [
  {
    label: 'Select',
    value: 'select',
    description: [
      'Selects soldiers on the floor to edit them, moves, copies and removes them.',
      'Clicking a soldier again selects the next soldier of the tile. Drag to move it, ' +
        'Ctrl+drag (Cmd+drag on macOS) to copy it. Delete removes the selected soldier, Esc ' +
        'deselects it. 1 to 9 set its patrol points at the cursor, Shift+1 to 9 removes them.',
    ],
  },
  {
    label: 'Place',
    value: 'place',
    description: [
      'Places soldiers of the team on the floor, on the roof floor only on flat roofs. Remove ' +
        'removes the soldiers of the teams shown in the layers on the floor on the tiles of the ' +
        'brush.',
    ],
  },
];

export interface SoldiersPanelProps {
  state: SoldierModeState;
  dispatch: Dispatch<SoldierModeAction>;
  /** The choices of the panel, `null` until the map is loaded */
  palette: SoldierPaletteEvent | null;
  /** The selected soldier and the counts, `null` until the viewer sent them */
  soldierEvent: EditorSoldierEvent | null;
  /** Why the last edit did not happen */
  notice: string | null;
  onDismissNotice: () => unknown;
}

/** The soldiers mode: new soldiers and the selected soldier */
export function SoldiersPanel({
  state,
  dispatch,
  palette,
  soldierEvent,
  notice,
  onDismissNotice,
}: SoldiersPanelProps) {
  const counts = soldierEvent?.counts ?? null;
  const selected = soldierEvent?.soldier ?? null;
  const maxTeam = palette?.maxTeamSoldiers ?? 32;
  const teamOptions = useMemo(
    () =>
      SOLDIER_TEAMS.map((team) => ({
        value: team,
        label: counts
          ? `${TEAM_LABELS[team]} ${counts[team]}/${maxTeam}`
          : TEAM_LABELS[team],
      })),
    [counts, maxTeam],
  );
  const onToolChange = useCallback(
    (tool: SoldierTool) => dispatch({ type: 'setTool', tool }),
    [dispatch],
  );
  const onEraseChange = useCallback(
    (erase: boolean) => dispatch({ type: 'setErase', erase }),
    [dispatch],
  );
  const onBrushChange = useCallback(
    (brush: BrushKind) => dispatch({ type: 'setBrush', brush }),
    [dispatch],
  );
  const onSizeChange = useCallback(
    (size: number) => dispatch({ type: 'setSize', size }),
    [dispatch],
  );
  const onTeamChange = useCallback(
    (team: SoldierTeam) => dispatch({ type: 'setTeam', team }),
    [dispatch],
  );

  let tool;
  if (state.tool === 'select') {
    tool = (
      <Flex vertical gap="small">
        {selected ? (
          <SoldierEditor
            soldier={selected}
            palette={palette}
            schedule={soldierEvent?.schedule ?? null}
            pick={soldierEvent?.pick ?? null}
            dispatch={dispatch}
          />
        ) : (
          <Typography.Text type="secondary">
            No soldier selected
          </Typography.Text>
        )}
      </Flex>
    );
  } else if (state.erase) {
    tool = (
      <Flex vertical gap="small">
        <BrushControls
          brushes={BRUSHES}
          brush={state.brush}
          size={state.size}
          onBrushChange={onBrushChange}
          onSizeChange={onSizeChange}
        />
      </Flex>
    );
  } else {
    tool = (
      <Flex vertical gap="small">
        <Segmented<SoldierTeam>
          block
          size="small"
          options={teamOptions}
          value={state.team}
          onChange={onTeamChange}
          aria-label="team"
        />
      </Flex>
    );
  }
  return (
    <Flex vertical gap="small">
      <ToolSelect<SoldierTool>
        tools={TOOLS}
        value={state.tool}
        onChange={onToolChange}
        label="soldier tool"
      />
      {state.tool === 'place' ? (
        <EraseControl erase={state.erase} onChange={onEraseChange} />
      ) : null}
      {counts ? (
        <Typography.Text type="secondary">
          {`${counts.total}/${palette?.maxSoldiers ?? 148} soldiers on the map`}
        </Typography.Text>
      ) : null}
      {notice ? (
        <Alert
          type="warning"
          showIcon
          closable
          message={notice}
          onClose={onDismissNotice}
        />
      ) : null}
      {tool}
    </Flex>
  );
}
