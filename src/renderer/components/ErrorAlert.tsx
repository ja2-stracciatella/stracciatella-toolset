import { SerializedError } from '@reduxjs/toolkit';
import { Alert, Typography } from 'antd';

interface ErrorAlertProps {
  error: { message?: string } | null;
}

export function ErrorAlert({ error }: ErrorAlertProps) {
  if (!error) return null;
  return (
    <Typography.Paragraph>
      <Alert type="error" message={error.message ?? ''} />
    </Typography.Paragraph>
  );
}
