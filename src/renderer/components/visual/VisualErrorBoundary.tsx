import { PropsWithChildren, useCallback } from 'react';
import { ErrorBoundary, FallbackProps } from 'react-error-boundary';
import { ErrorAlert } from '../common/ErrorAlert';
import { Button, Flex, Typography } from 'antd';
import { VisualFormProps } from './VisualFormWrapper';
import { useFileEditModeUpdate } from '../../hooks/useFileEditModeUpdate';

function Fallback({
  file,
  error,
  resetErrorBoundary,
}: FallbackProps & VisualFormProps) {
  const updateEditMode = useFileEditModeUpdate(file);
  const handleEditModeText = useCallback(() => {
    updateEditMode('text');
  }, [updateEditMode]);

  return (
    <Flex vertical gap="middle">
      <ErrorAlert error={error} />
      <Typography.Paragraph>
        An error occured in the visual editor. Usually this means your JSON file
        does not meet the expectations. To fix it, you can edit the JSON file in
        text mode.
      </Typography.Paragraph>
      <Flex gap="middle">
        <Button onClick={resetErrorBoundary}>Retry</Button>
        <Button onClick={handleEditModeText}>Switch to text edit mode</Button>
      </Flex>
    </Flex>
  );
}

export function VisualErrorBoundary({
  file,
  children,
}: PropsWithChildren<VisualFormProps>) {
  const FallbackComponent = useCallback(
    (props: FallbackProps) => <Fallback file={file} {...props} />,
    [file],
  );

  return (
    <ErrorBoundary FallbackComponent={FallbackComponent}>
      {children}
    </ErrorBoundary>
  );
}
