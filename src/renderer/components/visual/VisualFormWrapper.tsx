import { PropsWithChildren, useEffect, useMemo } from 'react';
import { EditorContent } from '../layout/EditorContent';
import { TextEditorOr } from '../TextEditor';
import { FullSizeLoader } from '../common/FullSizeLoader';
import { ErrorAlert, ErrorAlertProps } from '../common/ErrorAlert';
import { useAnyFileLoading } from '../../hooks/useAnyFileLoading';
import { useAnyFileLoadingError } from '../../hooks/useAnyFileLoadingError';
import { useFileLoad } from '../../hooks/useFileLoad';
import { useFileHasDiskValue } from '../../hooks/useFileHasDiskValue';
import { VisualErrorBoundary } from './VisualErrorBoundary';

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
  file,
  loading,
  children,
}: PropsWithChildren<Pick<VisualFormProps, 'file'> & { loading: boolean }>) {
  const hasDiskValue = useFileHasDiskValue(file);
  return loading && !hasDiskValue ? <FullSizeLoader /> : children;
}

function ErrorOr({ children, ...rest }: PropsWithChildren<ErrorAlertProps>) {
  return rest.error ? <ErrorAlert {...rest} /> : children;
}

export function VisualFormWrapper({
  file,
  extraFiles,
  children,
}: PropsWithChildren<VisualFormProps>) {
  const loadFile = useFileLoad();
  const allFiles = useMemo(
    () => [file, ...(extraFiles ?? [])],
    [file, extraFiles],
  );
  const loading = useAnyFileLoading(allFiles);
  const error = useAnyFileLoadingError(allFiles);

  useEffect(() => {
    for (const file of allFiles) {
      loadFile(file);
    }
  }, [allFiles, loadFile]);

  return (
    <EditorContent file={file}>
      <LoadingOr file={file} loading={loading}>
        <ErrorOr error={error}>
          <TextEditorOr file={file}>
            <VisualErrorBoundary file={file}>{children}</VisualErrorBoundary>
          </TextEditorOr>
        </ErrorOr>
      </LoadingOr>
    </EditorContent>
  );
}
