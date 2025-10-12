import { WidgetProps } from '@rjsf/utils';
import { Button, Col, Input, Row, Space } from 'antd';
import { useCallback, useState } from 'react';
import { invokeWithSchema } from '../../lib/invoke';
import z from 'zod';

export function HostPathWidget({ value, onChange }: WidgetProps) {
  const [modalIsOpen, setModalOpen] = useState(false);
  const openModal = useCallback(async () => {
    setModalOpen(true);
    const s = await invokeWithSchema(
      z.object({
        path: z.union([z.string(), z.null()]),
      }),
      'show_open_dialog',
      {
        type: 'open-directory',
      },
    );
    if (s.path) {
      onChange(s.path);
    }
    setModalOpen(false);
  }, [onChange]);

  return (
    <>
      <Row>
        <Col flex="auto">
          <Input
            style={{ flexGrow: 1 }}
            value={value}
            onChange={onChange}
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
