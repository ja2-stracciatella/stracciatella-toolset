import {
  HomeOutlined,
  FolderOpenOutlined,
  FolderOutlined,
  FileOutlined,
  ArrowUpOutlined,
} from '@ant-design/icons';
import { Button, List, Alert, Breadcrumb, Space } from 'antd';
import Modal from 'antd/lib/modal/Modal';
import { useCallback, useState, useEffect, useMemo } from 'react';
import { DirEntry, listDir, ResourceType } from '../../lib/listDir';

export function ResourceSelectorModal({
  resourceType,
  initialDir,
  isOpen,
  onSelect,
  onCancel,
}: {
  resourceType: ResourceType;
  initialDir?: string;
  isOpen: boolean;
  onSelect: (value: string) => unknown;
  onCancel: () => unknown;
}) {
  const [currentDir, setCurrentDir] = useState(initialDir ?? '');
  const [selectedEntry, setSelectedEntry] = useState<DirEntry | null>(null);
  const [entries, setEntries] = useState<Array<DirEntry> | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const modalOnOk = useCallback(() => {
    if (selectedEntry !== null && selectedEntry.type === 'File') {
      if (currentDir.length === 0) {
        onSelect(selectedEntry.path);
      } else {
        onSelect(`${currentDir}/${selectedEntry.path}`);
      }
    }
  }, [currentDir, onSelect, selectedEntry]);
  const onItemClick = useCallback(
    (entry: DirEntry) => {
      const absolutePath =
        currentDir.length === 0 ? entry.path : `${currentDir}/${entry.path}`;

      if (entry.type === 'Dir') {
        setSelectedEntry(null);
        setCurrentDir(absolutePath);
        return;
      }
      if (entry.path === selectedEntry?.path) {
        onSelect(absolutePath);
        return;
      }

      setSelectedEntry(entry);
    },
    [currentDir, onSelect, selectedEntry]
  );
  const setToParentDir = useCallback(() => {
    if (currentDir === '') {
      return;
    }
    setCurrentDir(currentDir.split('/').slice(0, -1).join('/'));
  }, [currentDir]);
  const breadcrumbs = useMemo(() => {
    return currentDir.split('/').reduce(
      (prev, curr, currIndex, array) => {
        if (array.length === 1 && curr === '') {
          return prev;
        }
        const path = array.slice(0, currIndex + 1).join('/');
        return [
          ...prev,
          <Breadcrumb.Item>
            <Button onClick={() => setCurrentDir(path)}>
              <FolderOpenOutlined />
              &nbsp;&nbsp;{curr}
            </Button>
          </Breadcrumb.Item>,
        ];
      },
      [
        <Breadcrumb.Item>
          <Button onClick={() => setCurrentDir('')}>
            <HomeOutlined />
          </Button>
        </Breadcrumb.Item>,
      ]
    );
  }, [currentDir]);
  const content = useMemo(() => {
    if (error) {
      return <Alert type="error" message={error.toString()} />;
    }
    const appliedEntries =
      entries !== null && entries.length > 0 ? entries : null;

    return (
      <List loading={!entries}>
        {appliedEntries?.map((entry, idx) => (
          <List.Item key={idx}>
            <Button
              type="link"
              onClick={(ev) => {
                onItemClick(entry);
                ev.preventDefault();
              }}
            >
              <Space>
                <span>
                  {entry.type === 'File' ? (
                    <FileOutlined style={{ fontSize: '150%' }} />
                  ) : (
                    <FolderOutlined style={{ fontSize: '150%' }} />
                  )}
                </span>
                <span>{entry.path}</span>
              </Space>
            </Button>
          </List.Item>
        ))}
      </List>
    );
  }, [entries, error, onItemClick]);

  useEffect(() => {
    setEntries(null);
    listDir(resourceType, currentDir)
      .then((e) => setEntries(e))
      .catch((e: any) => setError(e));
  }, [currentDir, resourceType]);

  return (
    <Modal
      title="Select resource"
      visible={isOpen}
      onOk={modalOnOk}
      onCancel={onCancel}
      width="calc(100vw - 400px)"
      bodyStyle={{
        overflowY: 'auto',
        overflowX: 'hidden',
        height: 'calc(100vh - 480px)',
      }}
    >
      <div
        style={{ display: 'flex', flexDirection: 'column', maxHeight: '100%' }}
      >
        <div>
          <Space direction="vertical">
            <Space>
              <Button onClick={setToParentDir} disabled={currentDir === ''}>
                <ArrowUpOutlined />
              </Button>
              <Breadcrumb>{breadcrumbs}</Breadcrumb>
            </Space>{' '}
          </Space>
        </div>
        <div style={{ flexGrow: 1, overflowY: 'auto' }}>{content}</div>
      </div>
    </Modal>
  );
}
