import { useAppDispatch, useAppSelector } from '../../hooks/state';
import { selectCloseModRequested } from '../../state/selectors';
import { Button, List, Modal, Typography } from 'antd';
import { setCloseRequested } from '../../state/toolset';
import { persistJSON, selectModifiedFiles } from '../../state/files';
import { invoke } from '../../lib/invoke';
import { useCallback, useEffect, useMemo } from 'react';

export function ConfirmCloseMod() {
  const dispatch = useAppDispatch();
  const closeRequested = useAppSelector(selectCloseModRequested);
  const modifiedFiles = useAppSelector(selectModifiedFiles);
  const cancel = useCallback(() => {
    dispatch(setCloseRequested(false));
  }, [dispatch]);
  const close = useCallback(async () => {
    await invoke('toolset/closeWindow', null);
  }, []);
  const saveAll = useCallback(async () => {
    for (const file of modifiedFiles) {
      dispatch(persistJSON(file));
    }
  }, [dispatch, modifiedFiles]);
  const footer = useMemo(() => {
    return (
      <>
        <Button onClick={cancel}>Cancel</Button>
        <Button onClick={close}>Discard Changes</Button>
        <Button type="primary" onClick={saveAll}>
          Save All
        </Button>
      </>
    );
  }, [cancel, close, saveAll]);

  useEffect(() => {
    if (closeRequested) {
      if (modifiedFiles.length == 0) {
        close();
      }
    }
  }, [close, closeRequested, modifiedFiles]);

  return (
    <Modal open={closeRequested} onCancel={cancel} footer={footer}>
      <Typography.Paragraph>
        You have unsaved changes in the following files:
      </Typography.Paragraph>
      <Typography.Paragraph>
        <List
          dataSource={modifiedFiles}
          renderItem={(item) => <List.Item>{item}</List.Item>}
          bordered
        />
      </Typography.Paragraph>
      <Typography.Paragraph>
        Please choose how to continue.
      </Typography.Paragraph>
    </Modal>
  );
}
