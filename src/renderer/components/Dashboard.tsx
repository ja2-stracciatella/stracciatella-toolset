import { Typography } from 'antd';
import { EditorContent } from './layout/EditorContent';
import { useAppSelector } from '../hooks/state';

export function Dashboard() {
  const modName = useAppSelector(
    (state) => state.mods.selected.data?.name || 'Unknown',
  );

  return (
    <EditorContent>
      <Typography.Title level={2}>Dashboard</Typography.Title>
      <Typography.Title level={3}>
        Welcome to the Stracciatella Toolset!
      </Typography.Title>
      <Typography.Paragraph>
        You are editing the mod: &quot;{modName}&quot;
      </Typography.Paragraph>
      <Typography.Paragraph>
        On the left you find a menu to select the different parts of the game
        that currently can be edited. Feel free to explore!
      </Typography.Paragraph>
    </EditorContent>
  );
}
