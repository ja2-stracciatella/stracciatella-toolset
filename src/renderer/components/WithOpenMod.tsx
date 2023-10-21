import { PropsWithChildren, useEffect } from 'react';
import './WithOpenMod.css';
import { OpenMod } from './OpenMod';
import { FullSizeLoader } from './FullSizeLoader';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { getMods } from '../state/mods';

export function WithOpenMod({ children }: PropsWithChildren<{}>) {
  const dispatch = useAppDispatch();
  const loading = useAppSelector((s) => s.mods.loading);
  const selectedMod = useAppSelector((s) => s.mods.selected);

  useEffect(() => {
    dispatch(getMods());
  }, [dispatch]);

  if (loading) {
    return (
      <div>
        <FullSizeLoader />
      </div>
    );
  }
  if (!selectedMod) {
    return <OpenMod />;
  }
  return <div>{children}</div>;
}
