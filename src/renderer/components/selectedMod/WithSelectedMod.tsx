import { PropsWithChildren, useEffect } from 'react';
import { SelectMod } from './SelectMod';
import { FullSizeLoader } from '../common/FullSizeLoader';
import { FullSizeDialogLayout } from '../layout/FullSizeDialogLayout';
import { ErrorAlert } from '../common/ErrorAlert';
import { useSelectedMod } from '../../hooks/useSelectedMod';
import { ConfirmCloseMod } from './ConfirmCloseMod';

export function WithSelectedMod({ children }: PropsWithChildren<unknown>) {
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
    return <SelectMod />;
  }
  return (
    <>
      <ConfirmCloseMod />
      {children}
    </>
  );
}
