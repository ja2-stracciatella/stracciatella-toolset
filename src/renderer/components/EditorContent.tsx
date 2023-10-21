import { ExclamationCircleOutlined, SaveOutlined } from '@ant-design/icons';
import { Button, Space } from 'antd';
import { ReactNode, memo, useCallback, useMemo } from 'react';
import './EditorContent.css';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { saveJSON } from '../state/files';

interface ContentProps {
  path: string;
  children: ReactNode;
}

const EditorContentHeader = memo(({ path }: { path: string }) => {
  const dispatch = useAppDispatch();
  const modified = useAppSelector((s) => s.files.json[path]?.content?.modified);
  const error = useAppSelector((s) => s.files.json[path]?.error);
  const errorStyle = useMemo(() => ({ color: '#9d1e1c' }), []);
  const saveFileToPath = useCallback(() => {
    dispatch(saveJSON(path));
  }, [dispatch, path]);
  return (
    <Space>
      <Button disabled={!modified}>
        <SaveOutlined onClick={saveFileToPath} />
      </Button>
      {error ? (
        <ExclamationCircleOutlined
          title={error.toString()}
          style={errorStyle}
        />
      ) : null}
    </Space>
  );
});

export function EditorContent({ path, children }: ContentProps) {
  return (
    <div className="editor-content">
      <div className="editor-content-header">
        <EditorContentHeader path={path} />
      </div>
      <div className="editor-content-content">{children}</div>
    </div>
  );
}
