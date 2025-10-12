import { useCallback, useMemo, useState } from 'react';
import { Button, Typography } from 'antd';
import { IChangeEvent } from '@rjsf/core';
import { JsonSchemaForm } from './JsonSchemaForm';
import z from 'zod';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { createNewMod, Mod } from '../state/mods';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { Space } from 'antd/lib';
import { ErrorAlert } from './ErrorAlert';

const NEW_MOD_SCHEMA = {
  type: 'object',
  properties: {
    id: {
      title: 'Mod ID',
      description:
        'This is used as the identifier and directory name for your mod. Must contain only lowercase letters, numbers and dashes.',
      type: 'string',
      minLength: 1,
      pattern: '^[a-z0-9\-]+$',
    },
    name: {
      title: 'Mod Name',
      description: 'The name that is displayed to the user for your mod.',
      type: 'string',
      minLength: 1,
    },
    description: {
      title: 'Description',
      description: 'A brief description of your mod.',
      type: 'string',
      minLength: 1,
    },
    version: {
      title: 'Version',
      description: 'A version for your mod. E.g. `0.1.0`',
      type: 'string',
      minLength: 1,
    },
  },
  required: ['id', 'name', 'version'],
};

interface NewModProps {
  onCancel: () => void;
}

export function NewMod({ onCancel }: NewModProps) {
  const dispatch = useAppDispatch();
  const stracciatellaHome = useAppSelector(
    (s) => s.toolset.config.value.stracciatellaHome,
  );
  const error = useAppSelector((s) => s.mods.error);
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
      dispatch(createNewMod(formData));
    }
  }, [dispatch, formData, valid]);

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
          schema={NEW_MOD_SCHEMA}
          content={formData}
          onChange={handleChange}
          onSubmit={handleSubmit}
        />
        <Space>
          <Button onClick={onCancel}>Cancel</Button>
          <Button type="primary" onClick={handleSubmit} disabled={!valid}>
            Submit
          </Button>
        </Space>
      </div>
    </FullSizeDialogLayout>
  );
}
