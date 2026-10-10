import { ExclamationCircleOutlined, SaveOutlined } from '@ant-design/icons';
import { Button, Flex, Select, theme, Typography } from 'antd';
import { ReactNode, useCallback, useMemo } from 'react';
import { useHotkeys } from 'react-hotkeys-hook';
import { useFileModified } from '../../hooks/useFileModified';
import { useFileSaving } from '../../hooks/useFileSaving';
import { useFileLoading } from '../../hooks/useFileLoading';
import { useFileSavingError } from '../../hooks/useFileSavingError';
import { useFileEditMode } from '../../hooks/useFileEditMode';
import { useFileEditModeUpdate } from '../../hooks/useFileEditModeUpdate';
import { useFileSaveMode } from '../../hooks/useFileSaveMode';
import { useFileSaveModeUpdate } from '../../hooks/useFileSaveModeUpdate';
import { useFileSave } from '../../hooks/useFileSave';
import { Loader } from '../common/Loader';

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
  file?: string;
  /** Shown above the content like the header of a file, e.g. a `SaveButton` */
  header?: ReactNode;
  children: ReactNode;
}

interface SaveButtonProps {
  /** Whether there is anything to save */
  modified: boolean;
  saving: boolean;
  error: { message?: string } | null;
  onSave: () => void;
}

/** Saves with a click or Ctrl+S while `modified` and not `saving` */
export function SaveButton({
  modified,
  saving,
  error,
  onSave,
}: SaveButtonProps) {
  const {
    token: { colorError },
  } = theme.useToken();
  const errorStyle = useMemo(() => ({ color: colorError }), [colorError]);
  const save = useCallback(() => {
    if (modified && !saving) {
      onSave();
    }
  }, [modified, onSave, saving]);

  useHotkeys('ctrl+s', save, {
    enableOnFormTags: true,
    preventDefault: true,
  });

  return (
    <Flex gap="small" align="center">
      <Button aria-label="Save" disabled={!modified || saving} onClick={save}>
        {saving ? <Loader /> : <SaveOutlined />}
      </Button>
      {error ? (
        <ExclamationCircleOutlined title={error.message} style={errorStyle} />
      ) : null}
    </Flex>
  );
}

function EditorContentSaveButton({ file }: { file: string }) {
  const modified = useFileModified(file);
  const saving = useFileSaving(file);
  const loading = useFileLoading(file);
  const error = useFileSavingError(file);
  const save = useFileSave(file);

  return (
    <SaveButton
      modified={modified && !loading}
      saving={saving}
      error={error}
      onSave={save}
    />
  );
}

function EditModeSelect({
  file,
}: Parameters<typeof EditorContentSaveButton>[0]) {
  const loading = useFileLoading(file);
  const saving = useFileSaving(file);
  const editMode = useFileEditMode(file);
  const update = useFileEditModeUpdate(file);

  return (
    <Flex gap="small" align="center">
      <Typography>Edit Mode</Typography>
      <Select
        style={{ minWidth: '150px' }}
        options={EDIT_MODE_SELECT_OPTIONS}
        value={editMode}
        onChange={update}
        disabled={loading || saving}
      />
    </Flex>
  );
}

function SaveModeSelect({
  file,
}: Parameters<typeof EditorContentSaveButton>[0]) {
  const loading = useFileLoading(file);
  const saving = useFileSaving(file);
  const editMode = useFileSaveMode(file);
  const update = useFileSaveModeUpdate(file);

  return (
    <Flex gap="small" align="center">
      <Typography>Save Mode</Typography>
      <Select
        style={{ minWidth: '150px' }}
        options={SAVE_MODE_SELECT_OPTIONS}
        value={editMode}
        onChange={update}
        disabled={loading || saving}
      />
    </Flex>
  );
}

function EditorContentHeader({ file }: { file: string }) {
  return (
    <Flex justify="space-between">
      <EditorContentSaveButton file={file} />
      <Flex gap="middle">
        <EditModeSelect file={file} />
        <SaveModeSelect file={file} />
      </Flex>
    </Flex>
  );
}

export const EditorContent = function EditorContent({
  file,
  header: customHeader,
  children,
}: ContentProps) {
  const flexStyle = useMemo(
    () =>
      ({
        height: '100%',
      }) as const,
    [],
  );
  const contentWrapperStyle = useMemo(
    () =>
      ({
        position: 'relative',
        margin: '10px',
        padding: '20px',
        flexGrow: 1,
        background: 'white',
        overflowY: 'auto',
      }) as const,
    [],
  );
  const contentStyle = useMemo(
    () =>
      ({
        position: 'relative',
        height: '1px',
        minHeight: '100%',
      }) as const,
    [],
  );
  const header = useMemo(() => {
    const content = file ? <EditorContentHeader file={file} /> : customHeader;
    if (!content) return null;
    return (
      <div
        style={{
          paddingTop: '10px',
          paddingLeft: '10px',
          paddingRight: '10px',
        }}
      >
        {content}
      </div>
    );
  }, [customHeader, file]);

  return (
    <Flex vertical style={flexStyle}>
      {header}
      <div style={contentWrapperStyle}>
        <div style={contentStyle}>{children}</div>
      </div>
    </Flex>
  );
};
