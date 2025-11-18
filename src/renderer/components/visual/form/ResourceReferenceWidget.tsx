import { WidgetProps } from '@rjsf/utils';
import { Input, Button, Flex } from 'antd';
import { useCallback, useMemo, useState } from 'react';
import { ResourceSelectorModal } from '../../content/ResourceSelectorModal';
import { SoundPreview } from '../../content/SoundPreview';
import { StiPreview } from '../../content/StiPreview';
import { ResourceType } from '../../../lib/resourceType';

interface ResourceReferenceWidgetProps extends WidgetProps {
  resourceType: ResourceType;
  pathPrefix: string[];
  postProcess: (path: string) => string;
  preview?: React.ReactNode;
}

export function ResourceReferenceWidget({
  resourceType,
  pathPrefix,
  postProcess,
  value,
  onChange,
  preview,
}: ResourceReferenceWidgetProps) {
  const [modalIsOpen, setModalOpen] = useState(false);
  const openModal = useCallback(() => setModalOpen(true), []);
  const closeModal = useCallback(() => setModalOpen(false), []);
  const onSelect = useCallback(
    (path: string) => {
      onChange(postProcess(path));
      closeModal();
    },
    [closeModal, onChange, postProcess],
  );
  const onInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange],
  );
  const previewElement = useMemo(() => {
    if (preview) {
      return preview;
    }
    const trimmed = (value ?? '').startsWith('/') ? value.substring(1) : value;
    if (resourceType === ResourceType.Sound) {
      return (
        <SoundPreview
          path={`${pathPrefix.join('/')}${pathPrefix.length > 0 ? '/' : ''}${trimmed}`}
        />
      );
    }
    if (resourceType === ResourceType.Graphics) {
      return (
        <StiPreview
          file={`${pathPrefix.join('/')}${pathPrefix.length > 0 ? '/' : ''}${trimmed}`}
        />
      );
    }
    return null;
  }, [preview, pathPrefix, resourceType, value]);

  return (
    <Flex dir="row" gap="small">
      {previewElement}
      <Input
        style={{ flexGrow: 1 }}
        value={value}
        onChange={onInputChange}
        placeholder="Please select a resource..."
      />
      <Button onClick={openModal}>…</Button>
      <ResourceSelectorModal
        isOpen={modalIsOpen}
        onSelect={onSelect}
        onCancel={closeModal}
        resourceType={resourceType}
        pathPrefix={pathPrefix}
      />
    </Flex>
  );
}

export function makeResourceReference({
  type = ResourceType.Any,
  prefix = [],
  postProcess = (path: string) => path,
}: {
  type: ResourceType;
  prefix?: string[];
  postProcess?: (path: string) => string;
}) {
  return function ResourceReference(props: WidgetProps) {
    return (
      <ResourceReferenceWidget
        resourceType={type}
        pathPrefix={prefix}
        postProcess={postProcess}
        {...props}
      />
    );
  };
}

export const resourceReferenceToSound = makeResourceReference({
  type: ResourceType.Sound,
});

export const resourceReferenceToGraphics = makeResourceReference({
  type: ResourceType.Graphics,
});
