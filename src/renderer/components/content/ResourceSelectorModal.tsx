import {
  HomeOutlined,
  FolderOutlined,
  FileOutlined,
  ArrowUpOutlined,
} from '@ant-design/icons';
import { Button, List, Breadcrumb, Flex, Splitter } from 'antd';
import Modal from 'antd/lib/modal/Modal';
import {
  useCallback,
  useState,
  useMemo,
  memo,
  FunctionComponent,
  useEffect,
} from 'react';
import { ErrorAlert } from '../ErrorAlert';
import { SoundPreview } from './SoundPreview';
import { useImageMetadata } from '../../hooks/useImageMetadata';
import { StiPreview } from './StiPreview';
import { DirEntry, useDirEntries } from '../../hooks/useDirEntries';
import { ResourceType, resourceTypeFromFilename } from '../../lib/resourceType';

const Breadcrumbs = memo(function Breadcrumbs({
  currentDir,
  switchDir,
}: {
  currentDir: string[];
  switchDir: (currentDir: string[]) => void;
}) {
  const setToParentDir = useCallback(() => {
    if (currentDir.length == 0) {
      return;
    }
    switchDir(currentDir.slice(0, -1));
  }, [currentDir, switchDir]);
  const items = useMemo(() => {
    return currentDir.reduce(
      (prev, curr, currIndex) => {
        const path = currentDir.slice(0, currIndex + 1);
        return [
          ...prev,
          {
            title: <a>{curr}</a>,
            onClick: () => switchDir(path),
          },
        ];
      },
      [
        {
          title: (
            <a>
              <HomeOutlined />
            </a>
          ),
          onClick: () => switchDir([]),
        },
      ],
    );
  }, [currentDir, switchDir]);

  return (
    <Flex gap="middle" align="center">
      <Button
        size="small"
        onClick={setToParentDir}
        disabled={currentDir.length === 0}
      >
        <ArrowUpOutlined />
      </Button>
      <Breadcrumb items={items} />
    </Flex>
  );
});

const GraphicsPreview = memo(function GraphicsPreview({
  path,
}: {
  path: string;
}) {
  const { data, error, refresh } = useImageMetadata(path);
  const previews = useMemo(() => {
    if (!data) {
      return null;
    }
    return data.images.map((_, i) => (
      <StiPreview key={i} file={path} subimage={i} />
    ));
  }, [data, path]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (error) {
    return <ErrorAlert error={error} />;
  }

  return (
    <Flex gap="middle" wrap>
      {previews}
    </Flex>
  );
});

const previewMapping: Record<
  ResourceType,
  FunctionComponent<{ path: string }>
> = {
  [ResourceType.Any]: () => <span>Cannot preview this file type.</span>,
  [ResourceType.Graphics]: GraphicsPreview,
  [ResourceType.Sound]: SoundPreview,
};

const Preview = memo(function Preview({
  dir,
  entry,
}: {
  dir: string;
  entry: DirEntry | null;
}) {
  if (!entry || entry.type !== 'File') {
    return <span>Select file for preview</span>;
  }
  const resourceType = resourceTypeFromFilename(entry.path);
  const PreviewComponent = previewMapping[resourceType];
  return (
    <PreviewComponent
      path={`${dir}${dir.length > 0 ? '/' : ''}${entry.path}`}
    />
  );
});

export function ResourceSelectorModal({
  resourceType,
  initialDir,
  isOpen,
  onSelect,
  onCancel,
}: {
  resourceType: ResourceType;
  initialDir?: string[];
  isOpen: boolean;
  onSelect: (value: string) => unknown;
  onCancel: () => unknown;
}) {
  const [currentDir, setCurrentDir] = useState(initialDir ?? []);
  const [selectedEntry, setSelectedEntry] = useState<DirEntry | null>(null);
  const switchDir = useCallback((dir: string[]) => {
    setCurrentDir(dir);
    setSelectedEntry(null);
  }, []);
  const {
    data: entries,
    loading,
    error,
    refresh,
  } = useDirEntries(currentDir.join('/'), resourceType);
  const modalOnOk = useCallback(() => {
    if (selectedEntry !== null && selectedEntry.type === 'File') {
      if (currentDir.length === 0) {
        onSelect(selectedEntry.path);
      } else {
        onSelect(`${currentDir}/${selectedEntry.path}`);
      }
    }
  }, [currentDir, onSelect, selectedEntry]);
  const modalOnCancel = useCallback(() => {
    setSelectedEntry(null);
    onCancel();
  }, [onCancel]);
  const onItemClick = useCallback(
    (entry: DirEntry) => {
      const newDir = [...currentDir, entry.path];

      if (entry.type === 'Dir') {
        switchDir(newDir);
        return;
      }
      if (entry.path === selectedEntry?.path) {
        onSelect(newDir.join('/'));
        return;
      }

      setSelectedEntry(entry);
    },
    [currentDir, onSelect, selectedEntry, switchDir],
  );
  const renderItem = useCallback(
    (entry: DirEntry) => {
      return (
        <List.Item>
          <Button
            onClick={(ev) => {
              ev.preventDefault();
              onItemClick(entry);
            }}
            type={selectedEntry?.path === entry.path ? 'primary' : 'default'}
            disabled={loading}
            icon={
              entry.type === 'Dir' ? (
                <FolderOutlined size={32} />
              ) : (
                <FileOutlined size={32} />
              )
            }
            block
            style={{
              textOverflow: 'ellipsis',
              overflowX: 'hidden',
              justifyContent: 'left',
            }}
          >
            {entry.path}
          </Button>
        </List.Item>
      );
    },
    [loading, onItemClick, selectedEntry],
  );
  const content = useMemo(() => {
    if (error) {
      return <ErrorAlert error={error} />;
    }
    return (
      <List
        dataSource={entries ?? []}
        renderItem={renderItem}
        grid={{ gutter: 16, column: 4 }}
      />
    );
  }, [entries, error, renderItem]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <Modal
      title="Select resource"
      open={isOpen}
      onOk={modalOnOk}
      onCancel={modalOnCancel}
      confirmLoading={!entries}
      width="80%"
      styles={{ body: { overflow: 'hidden', height: '60vh' } }}
    >
      <Flex vertical gap="middle" style={{ height: '100%' }}>
        <Breadcrumbs currentDir={currentDir} switchDir={switchDir} />
        <div style={{ flexGrow: 1, overflowY: 'auto' }}>
          <Splitter>
            <Splitter.Panel
              size="70%"
              resizable={false}
              style={{ overflowX: 'hidden' }}
            >
              {content}
            </Splitter.Panel>
            <Splitter.Panel size="30%" resizable={false}>
              <Flex
                justify="center"
                align="center"
                style={{ height: '100%', padding: '10px' }}
              >
                <Preview dir={currentDir.join('/')} entry={selectedEntry} />
              </Flex>
            </Splitter.Panel>
          </Splitter>
        </div>
      </Flex>
    </Modal>
  );
}
