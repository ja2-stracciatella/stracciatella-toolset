import { PropsWithChildren, useEffect } from 'react';
import { OpenMod } from './OpenMod';
import { FullSizeLoader } from './FullSizeLoader';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { ErrorAlert } from './ErrorAlert';
import { useSelectedMod } from '../hooks/useSelectedMod';

export function WithOpenMod({ children }: PropsWithChildren<unknown>) {
  const { loading, loadingError, data, refresh } = useSelectedMod();

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (loadingError) {
    return (
      <FullSizeDialogLayout>
        <ErrorAlert error={loadingError} />
      </FullSizeDialogLayout>
    );
  }
  if (loading) {
    return <FullSizeLoader />;
  }
  if (!data) {
    return <OpenMod />;
  }
  return <>{children}</>;
}
