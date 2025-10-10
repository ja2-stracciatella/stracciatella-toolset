import { Typography } from 'antd';

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
  },
  required: ['id', 'name'],
};

export function NewMod() {
  return (
    <div className="new-mod-root">
      <Typography.Title>Create new mod</Typography.Title>
    </div>
  );
}
