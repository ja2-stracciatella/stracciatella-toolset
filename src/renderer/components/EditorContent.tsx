import { ExclamationCircleOutlined, SaveOutlined } from '@ant-design/icons';
import { Button, Flex, Select, Typography } from 'antd';
import { ReactNode, memo, useCallback, useMemo } from 'react';
import { useHotkeys } from 'react-hotkeys-hook';
import { useAppDispatch } from '../hooks/state';
import {
  changeSaveMode,
  changeEditMode,
  EditMode,
  persistJSON,
  SaveMode,
} from '../state/files';
import {
  useFileEditMode,
  useFileModified,
  useFilePersistingError,
  useFileSaveMode,
  useFileSaving,
} from '../hooks/files';

const SAVE_MODE_SELECT_OPTIONS = [
  {
    label: 'Patch',
    title:
      'Use this option if your mod should only adapt content, not fully replace it. The resulting file will only contain the changes you made to the original file.',
    value: 'patch',
  },
  {
    label: 'Replace',
    title:
      'Use this option if you want to fully replace content from vanilla or other mods. The resulting file will contain the modified original file.',
    value: 'replace',
  },
];

const EDIT_MODE_SELECT_OPTIONS = [
  {
    label: 'Visual',
    title: 'Use a visual editor to edit the file',
    value: 'visual',
  },
  {
    label: 'Text',
    title: 'Use a text editor to edit the file',
    value: 'text',
  },
];

interface ContentProps {
  path: string;
  children: ReactNode;
}

const EditorContentHeader = memo(function EditorContentHeader({
  path,
}: {
  path: string;
}) {
  const dispatch = useAppDispatch();
  const modified = useFileModified(path);
  const saving = useFileSaving(path);
  const saveMode = useFileSaveMode(path);
  const editMode = useFileEditMode(path);
  const error = useFilePersistingError(path);
  const errorStyle = useMemo(() => ({ color: '#9d1e1c' }), []);
  const saveFile = useCallback(() => {
    dispatch(persistJSON(path));
  }, [dispatch, path]);
  const setSaveMode = useCallback(
    (saveMode: SaveMode) => {
      dispatch(
        changeSaveMode({
          filename: path,
          saveMode,
        }),
      );
    },
    [dispatch, path],
  );
  const setEditMode = useCallback(
    (editMode: EditMode) => {
      dispatch(
        changeEditMode({
          filename: path,
          editMode,
        }),
      );
    },
    [dispatch, path],
  );

  useHotkeys('ctrl+s', saveFile, {
    enableOnFormTags: true,
    preventDefault: true,
  });

  return (
    <Flex justify="space-between">
      <Flex gap="small">
        <Button disabled={!modified || !!saving}>
          <SaveOutlined onClick={saveFile} />
        </Button>
        {error ? (
          <ExclamationCircleOutlined title={error.message} style={errorStyle} />
        ) : null}
      </Flex>
      <Flex gap="middle">
        <Flex gap="small" align="center">
          <Typography>Edit Mode</Typography>
          <Select
            style={{ minWidth: '150px' }}
            options={EDIT_MODE_SELECT_OPTIONS}
            value={editMode}
            onChange={setEditMode}
          />
        </Flex>
        <Flex gap="small" align="center">
          <Typography>Save Mode</Typography>
          <Select
            style={{ minWidth: '150px' }}
            options={SAVE_MODE_SELECT_OPTIONS}
            value={saveMode}
            onChange={setSaveMode}
          />
        </Flex>
      </Flex>
    </Flex>
  );
});

export const EditorContent = memo(function EditorContent({
  path,
  children,
}: ContentProps) {
  return (
    <Flex vertical style={{ height: '100%' }}>
      <div
        style={{
          paddingTop: '10px',
          paddingLeft: '10px',
          paddingRight: '10px',
        }}
      >
        <EditorContentHeader path={path} />
      </div>
      <div
        style={{
          position: 'relative',
          margin: '10px',
          padding: '20px',
          flexGrow: 1,
          background: 'white',
          overflowY: 'auto',
        }}
      >
        <div style={{ position: 'relative', height: '1px', minHeight: '100%' }}>
          {children}
        </div>
      </div>
    </Flex>
  );
});
