import { WidgetProps } from '@rjsf/utils';
import { Button, Col, Input, Row, Space } from 'antd';
import { useCallback, useState } from 'react';
import { invoke } from '../../lib/invoke';

export function HostPathWidget({ value, onChange }: WidgetProps) {
  const [modalIsOpen, setModalOpen] = useState(false);
  const openModal = useCallback(async () => {
    setModalOpen(true);
    const s = await invoke('dialog/showOpenDialog', {
      type: 'open-directory',
    });
    if (s.path) {
      onChange(s.path);
    }
    setModalOpen(false);
  }, [onChange]);
  const onInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange],
  );

  return (
    <>
      <Row>
        <Col flex="auto">
          <Input
            style={{ flexGrow: 1 }}
            value={value}
            onChange={onInputChange}
            placeholder="Please select a resource..."
          />
        </Col>
        <Col flex="none">
          <Space>
            {' '}
            <Button onClick={openModal} disabled={modalIsOpen}>
              …
            </Button>
          </Space>
        </Col>
      </Row>
    </>
  );
}
