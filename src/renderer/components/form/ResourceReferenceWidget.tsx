import { WidgetProps } from '@rjsf/utils';
import { Input, Button, Space, Row, Col } from 'antd';
import { useCallback, useMemo, useState } from 'react';
import { ResourceType } from '../../lib/listDir';
import { ResourceSelectorModal } from '../content/ResourceSelectorModal';
import { SoundPreview } from '../content/SoundPreview';

interface ResourceReferenceWidgetProps extends WidgetProps {
  resourceType: ResourceType;
}

export function ResourceReferenceWidget({
  resourceType,
  value,
  onChange,
}: ResourceReferenceWidgetProps) {
  const [modalIsOpen, setModalOpen] = useState(false);
  const openModal = useCallback(() => setModalOpen(true), []);
  const closeModal = useCallback(() => setModalOpen(false), []);
  const onSelect = useCallback(
    (path: string) => {
      onChange(path);
      closeModal();
    },
    [closeModal, onChange],
  );
  const preview = useMemo(() => {
    let element = null;
    if (resourceType === ResourceType.Sound) {
      element = <SoundPreview path={value} />;
    }

    if (element === null) {
      return null;
    }
    return (
      <Col flex="none">
        <Space>{element} </Space>
      </Col>
    );
  }, [resourceType, value]);

  return (
    <>
      <Row>
        {preview}
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
            <Button onClick={openModal}>…</Button>
          </Space>
        </Col>
      </Row>
      <ResourceSelectorModal
        isOpen={modalIsOpen}
        onSelect={onSelect}
        onCancel={closeModal}
        resourceType={resourceType}
      />
    </>
  );
}

export function resourceReference(type: ResourceType = ResourceType.Any) {
  return function ResourceReference(props: WidgetProps) {
    return <ResourceReferenceWidget resourceType={type} {...props} />;
  };
}
