import { ExclamationOutlined } from '@ant-design/icons';
import { Image, Flex, Spin } from 'antd';
import { useEffect, useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';
import { useFileJson } from '../../hooks/files';

interface MercPreviewProps {
  profile: string;
}

export function MercPreview({ profile }: MercPreviewProps) {
  const [content] = useFileJson('mercs-profile-info.json');
  const profileId = useMemo(() => {
    if (!content) {
      return null;
    }
    if (!Array.isArray(content)) {
      return null;
    }
    const p = content.find((it: any) => it.internalName === profile);
    if (!p || !p.profileID) {
      return null;
    }
    return p.profileID as number;
  }, [content, profile]);
  const graphic1 = useMemo(() => {
    if (profileId === null) {
      return null;
    }
    return `faces/${profileId.toString().padStart(2, '0')}.sti`;
  }, [profileId]);
  const graphic2 = useMemo(() => {
    if (profileId === null) {
      return null;
    }
    return `faces/B${profileId.toString().padStart(2, '0')}.sti`;
  }, [profileId]);
  const {
    loading: loading1,
    data: image1,
    error: error1,
    refresh: refresh1,
  } = useImageFile(graphic1);
  const {
    loading: loading2,
    data: image2,
    error: error2,
    refresh: refresh2,
  } = useImageFile(graphic2);
  const image = useMemo(() => {
    const loading = loading1 || loading2;
    if (loading) {
      return <Spin size="small" />;
    }
    const error = error1 && error2 ? error1 || error2 : null;
    if (error) {
      return <ExclamationOutlined title={error.message} />;
    }
    if (!image1 && !image2) {
      return null;
    }
    return <Image preview={false} src={(image1 || image2) ?? undefined} />;
  }, [error1, error2, image1, image2, loading1, loading2]);

  useEffect(() => {
    refresh1();
    refresh2();
  }, [refresh1, refresh2]);

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
