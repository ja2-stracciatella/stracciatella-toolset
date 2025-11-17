import { PropsWithChildren, useEffect, useMemo } from 'react';
import { EditorContent } from '../layout/EditorContent';
import { TextEditorOr } from '../TextEditor';
import { FullSizeLoader } from '../common/FullSizeLoader';
import { ErrorAlert, ErrorAlertProps } from '../common/ErrorAlert';
import { useAnyFileLoading } from '../../hooks/useAnyFileLoading';
import { useAnyFileLoadingError } from '../../hooks/useAnyFileLoadingError';
import { useAppDispatch } from '../../hooks/state';
import { loadJSON } from '../../state/files';

export interface VisualFormProps {
  /*
   * The file that the form edits
   */
  file: string;
  /*
   * Other files that need to be loaded to display the form
   * These files are loaded in parallel to form, but dont affect e.g. the modified state
   */
  extraFiles?: string[];
}

function LoadingOr({
  children,
  loading,
}: PropsWithChildren<{ loading: boolean }>) {
  return loading ? <FullSizeLoader /> : children;
}

function ErrorOr({ children, ...rest }: PropsWithChildren<ErrorAlertProps>) {
  return rest.error ? <ErrorAlert {...rest} /> : children;
}

export function VisualFormWrapper({
  file,
  extraFiles,
  children,
}: PropsWithChildren<VisualFormProps>) {
  const dispatch = useAppDispatch();
  const allFiles = useMemo(
    () => [file, ...(extraFiles ?? [])],
    [file, extraFiles],
  );
  const loading = useAnyFileLoading(allFiles);
  const error = useAnyFileLoadingError(allFiles);

  useEffect(() => {
    for (const file of allFiles) {
      dispatch(loadJSON(file));
    }
  }, [dispatch, allFiles]);

  return (
    <EditorContent file={file}>
      <LoadingOr loading={loading}>
        <ErrorOr error={error}>
          <TextEditorOr file={file}>{children}</TextEditorOr>
        </ErrorOr>
      </LoadingOr>
    </EditorContent>
  );
}
