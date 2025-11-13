import { DeleteOutlined } from '@ant-design/icons';
import { Button, Modal } from 'antd';
import { MouseEventHandler, useCallback, useState } from 'react';
import { useAppDispatch } from '../../hooks/state';
import { removeJsonItem } from '../../state/files';

interface RemoveButtonProps {
  file: string;
  index: number;
  label?: string;
}

export function RemoveButton({ file, index, label }: RemoveButtonProps) {
  const dispatch = useAppDispatch();
  const [showModal, setShowModal] = useState(false);
  const handleClick: MouseEventHandler = useCallback((ev) => {
    setShowModal(true);
    ev.stopPropagation();
  }, []);
  const handleConfirm: MouseEventHandler = useCallback(
    (ev) => {
      dispatch(
        removeJsonItem({
          filename: file,
          index,
        }),
      );
      setShowModal(false);
      ev.stopPropagation();
    },
    [dispatch, file, index],
  );
  const handleCancel: MouseEventHandler = useCallback((ev) => {
    setShowModal(false);
    ev.stopPropagation();
  }, []);

  return (
    <>
      <Button icon={<DeleteOutlined />} onClick={handleClick}>
        {label ?? ''}
      </Button>
      <Modal
        title="Confirm"
        open={showModal}
        onOk={handleConfirm}
        onCancel={handleCancel}
      >
        Removing the item cannot be undone. Are you sure?
      </Modal>
    </>
  );
}
