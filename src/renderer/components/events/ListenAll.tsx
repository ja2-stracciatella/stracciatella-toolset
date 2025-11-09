import { Listen } from './Listen';
import { useCallback } from 'react';
import { invoke } from '../../lib/invoke';
import { useAppDispatch } from '../../hooks/state';
import { useSelectedMod } from '../../hooks/useSelectedMod';
import { setCloseRequested } from '../../state/toolset';

function ListenOnToolsetCloseWindowRequested() {
  const dispatch = useAppDispatch();
  const { data: selectedMod } = useSelectedMod();

  const onToolsetCloseWindowRequested = useCallback(() => {
    if (!selectedMod) {
      invoke('toolset/closeWindow', null);
    } else {
      dispatch(setCloseRequested(true));
    }
  }, [dispatch, selectedMod]);

  return (
    <Listen
      name="toolset/closeWindowRequested"
      callback={onToolsetCloseWindowRequested}
    />
  );
}

export function ListenAll() {
  return (
    <>
      <ListenOnToolsetCloseWindowRequested />
    </>
  );
}
