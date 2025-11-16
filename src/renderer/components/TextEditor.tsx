import { Editor, useMonaco, loader } from '@monaco-editor/react';
import {
  useFileEditMode,
  useFileSaveMode,
  useFileSchema,
  useFileText,
} from '../hooks/files';
import * as monaco from 'monaco-editor';
import { FullSizeLoader } from './common/FullSizeLoader';
import { useCallback, useEffect } from 'react';
import { toJSONSchema } from 'zod';
import { JSON_PATCH_SCHEMA } from '../../common/invokables/jsons';
import { useAppDispatch } from '../hooks/state';
import { changeText } from '../state/files';

const JSON_PATCH_JSON_SCHEMA = toJSONSchema(JSON_PATCH_SCHEMA);

const MONACO_OPTIONS = { fixedOverflowWidgets: true };

loader.config({ monaco });

export interface TextEditorProps {
  file: string;
}

export function TextEditor({ file }: TextEditorProps) {
  const dispatch = useAppDispatch();
  const monaco = useMonaco();
  const saveMode = useFileSaveMode(file);
  const schema = useFileSchema(file);
  const [text] = useFileText(file);
  const loading = !monaco || typeof text !== 'string';
  const onChange = useCallback(
    (value: string | undefined) => {
      if (typeof value === 'undefined') return;
      dispatch(
        changeText({
          filename: file,
          value,
        }),
      );
    },
    [dispatch, file],
  );

  useEffect(() => {
    if (monaco) {
      monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
        validate: true,
        schemas: [
          {
            uri: `http://ja2-stracciatella.github.io/${file}`,
            fileMatch: [`*`],
            schema: saveMode === 'replace' ? schema : JSON_PATCH_JSON_SCHEMA,
          },
        ],
      });
    }
  }, [file, monaco, saveMode, schema]);

  if (loading) {
    return <FullSizeLoader />;
  }
  return (
    <Editor
      path={file}
      value={text}
      onChange={onChange}
      options={MONACO_OPTIONS}
      language="json"
      height="100%"
    />
  );
}

export function TextEditorOr({
  file,
  children,
}: TextEditorProps & { children?: React.ReactNode }) {
  const editMode = useFileEditMode(file);

  if (editMode === 'text') {
    return <TextEditor file={file} />;
  }
  return children;
}
