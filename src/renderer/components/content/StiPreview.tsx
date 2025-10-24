import { ExclamationOutlined } from '@ant-design/icons';
import { Avatar, Flex, Image, Spin } from 'antd';
import { useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';

export function StiPreview({
  file,
  subimage,
}: {
  file: string;
  subimage?: number;
}) {
  const { loading, data, error } = useImageFile(file, subimage);
  const image = useMemo(() => {
    if (loading) {
      return <Spin size="small" />;
    }
    if (error) {
      return <ExclamationOutlined title={error.message} />;
    }
    if (!data) {
      return null;
    }
    return <Image preview={false} src={data} />;
  }, [loading, error, data]);

  return (
    <Flex
      style={{ width: '2em', height: '2em' }}
      justify="center"
      align="center"
    >
      {image}
    </Flex>
  );
}
