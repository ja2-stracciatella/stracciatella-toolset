import { WidgetProps } from '@rjsf/utils';
import { Input, Button, Flex } from 'antd';
import { useCallback, useMemo, useState } from 'react';
import { ResourceSelectorModal } from '../content/ResourceSelectorModal';
import { SoundPreview } from '../content/SoundPreview';
import { StiPreview } from '../content/StiPreview';
import { ResourceType } from '../../lib/resourceType';

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
    if (resourceType === ResourceType.Sound) {
      return <SoundPreview path={value} />;
    }
    if (resourceType === ResourceType.Graphics) {
      return <StiPreview file={value} />;
    }
    return null;
  }, [resourceType, value]);

  return (
    <Flex dir="row" gap="small">
      {preview}
      <Input
        style={{ flexGrow: 1 }}
        value={value}
        onChange={onChange}
        placeholder="Please select a resource..."
      />
      <Button onClick={openModal}>…</Button>
      <ResourceSelectorModal
        isOpen={modalIsOpen}
        onSelect={onSelect}
        onCancel={closeModal}
        resourceType={resourceType}
      />
    </Flex>
  );
}

function resourceReference(type: ResourceType = ResourceType.Any) {
  return function ResourceReference(props: WidgetProps) {
    return <ResourceReferenceWidget resourceType={type} {...props} />;
  };
}

export const resourceReferenceToSound = resourceReference(ResourceType.Sound);

export const resourceReferenceToGraphics = resourceReference(
  ResourceType.Graphics,
);
