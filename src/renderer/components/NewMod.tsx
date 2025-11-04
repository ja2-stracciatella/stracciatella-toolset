import { useCallback, useMemo, useState } from 'react';
import { Button, Typography } from 'antd';
import { IChangeEvent } from '@rjsf/core';
import { JsonSchemaForm } from './JsonSchemaForm';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { createNewMod, Mod } from '../state/mods';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { Space } from 'antd/lib';
import { ErrorAlert } from './ErrorAlert';
import { useSelectedMod } from '../hooks/useSelectedMod';
import { MOD_JSON_SCHEMA } from '../state/mods';

export function NewMod({ onCancel }: { onCancel: () => void }) {
  const {
    persisting: loading,
    persistingError: error,
    create,
  } = useSelectedMod();
  const stracciatellaHome = useAppSelector(
    (s) => s.toolset.data?.config.stracciatellaHome,
  );
  const [formData, setFormData] = useState<Mod>({
    id: '',
    name: '',
    version: '',
  });
  const [valid, setValid] = useState(false);
  const handleChange = useCallback((ev: IChangeEvent<any>) => {
    if (ev.formData) {
      setFormData(ev.formData);
      setValid(ev.errors.length === 0);
    }
  }, []);
  const createdModDir = useMemo(() => {
    if (!formData.id) return '';
    return `${stracciatellaHome ?? ''}/mods/${formData.id}`;
  }, [stracciatellaHome, formData.id]);

  const handleSubmit = useCallback(async () => {
    if (valid) {
      create(formData);
    }
  }, [create, formData, valid]);

  return (
    <FullSizeDialogLayout>
      <Typography.Title>Create new mod</Typography.Title>
      <Typography.Paragraph>
        This allows you to create a new mod. New mods are automatically placed
        in the mods folder.
      </Typography.Paragraph>
      <Typography.Paragraph>
        Your new mod will be placed at {createdModDir}
      </Typography.Paragraph>
      <ErrorAlert error={error} />
      <div>
        <JsonSchemaForm
          schema={MOD_JSON_SCHEMA}
          content={formData}
          onChange={handleChange}
          onSubmit={handleSubmit}
        />
        <Space>
          <Button onClick={onCancel}>Cancel</Button>
          <Button
            type="primary"
            onClick={handleSubmit}
            disabled={!valid || loading}
          >
            Submit
          </Button>
        </Space>
      </div>
    </FullSizeDialogLayout>
  );
}
