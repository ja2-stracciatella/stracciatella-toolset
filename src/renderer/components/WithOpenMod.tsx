import { PropsWithChildren, useEffect } from 'react';
import { OpenMod } from './OpenMod';
import { FullSizeLoader } from './FullSizeLoader';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { getMods } from '../state/mods';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { ErrorAlert } from './ErrorAlert';

export function WithOpenMod({ children }: PropsWithChildren<{}>) {
  const dispatch = useAppDispatch();
  const allMods = useAppSelector((s) => s.mods.mods);
  const error = useAppSelector((s) => s.mods.error);
  const selectedMod = useAppSelector((s) => s.mods.selected);

  useEffect(() => {
    dispatch(getMods());
  }, [dispatch]);

  if (!allMods && error) {
    return (
      <FullSizeDialogLayout>
        <ErrorAlert error={error} />
      </FullSizeDialogLayout>
    );
  }
  if (!allMods) {
    return <FullSizeLoader />;
  }
  if (!selectedMod) {
    return <OpenMod />;
  }
  return <>{children}</>;
}
