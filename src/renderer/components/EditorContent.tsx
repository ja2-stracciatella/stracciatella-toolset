import { ExclamationCircleOutlined, SaveOutlined } from '@ant-design/icons';
import { Button, Flex, Select, Space, Typography } from 'antd';
import { ReactNode, memo, useCallback, useMemo } from 'react';
import './EditorContent.css';
import { useAppDispatch } from '../hooks/state';
import { changeSaveMode, persistJSON, SaveMode } from '../state/files';
import {
  useFileError,
  useFileModified,
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
  const error = useFileError(path);
  const errorStyle = useMemo(() => ({ color: '#9d1e1c' }), []);
  const saveFileToPath = useCallback(() => {
    dispatch(
      persistJSON({
        filename: path,
      }),
    );
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

  return (
    <Flex justify="space-between">
      <Space>
        <Button disabled={!modified || !!saving}>
          <SaveOutlined onClick={saveFileToPath} />
        </Button>
        {error ? (
          <ExclamationCircleOutlined title={error.message} style={errorStyle} />
        ) : null}
      </Space>
      <Space>
        <Typography>Save Mode</Typography>
        <Select
          style={{ minWidth: '150px' }}
          options={SAVE_MODE_SELECT_OPTIONS}
          value={saveMode}
          onChange={setSaveMode}
        />
      </Space>
    </Flex>
  );
});

export const EditorContent = memo(function EditorContent({
  path,
  children,
}: ContentProps) {
  return (
    <div className="editor-content">
      <div className="editor-content-header">
        <EditorContentHeader path={path} />
      </div>
      <div className="editor-content-content">{children}</div>
    </div>
  );
});
