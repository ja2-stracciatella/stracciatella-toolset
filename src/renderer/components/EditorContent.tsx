import { ExclamationCircleOutlined, SaveOutlined } from '@ant-design/icons';
import { Button, Space } from 'antd';
import { ReactNode, useCallback, useMemo } from 'react';
import { useFiles } from '../state/files';

import './EditorContent.css';

interface ContentProps {
  path: string;
  children: ReactNode;
}

function EditorContentHeader({ path }: { path: string }) {
  const { saveFile, inMemory, saveErrors } = useFiles();
  const disabled = useMemo(() => !inMemory[path], [inMemory, path]);
  const error = useMemo(() => saveErrors[path], [path, saveErrors]);
  const errorStyle = useMemo(() => ({ color: '#9d1e1c' }), []);
  const saveFileToPath = useCallback(() => {
    saveFile(path);
  }, [path, saveFile]);
  return (
    <Space>
      <Button disabled={disabled}>
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
}

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

EditorContent.whyDidYouRender = true;
