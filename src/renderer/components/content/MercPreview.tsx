import { QuestionOutlined } from '@ant-design/icons';
import { Avatar } from 'antd';
import { useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';
import { useJson } from '../../hooks/files';

interface MercPreviewProps {
  profile: string;
}

const crispEdgesStyle = { imageRendering: 'crisp-edges' as const };

export function MercPreview({ profile }: MercPreviewProps) {
  const { content } = useJson('mercs-profile-info.json');
  const profileId = useMemo(() => {
    if (!content) {
      return null;
    }
    const p = content.value.find((it: any) => it.internalName === profile);
    if (!p) {
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
  const { data: image1, error: error1 } = useImageFile(graphic1);
  const { data: image2, error: error2 } = useImageFile(graphic2);
  const additionalAvatarProps = useMemo(() => {
    if (error1 && error2) {
      return { icon: <QuestionOutlined /> };
    }
    if (!image1 && !image2) {
      return { icon: <QuestionOutlined /> };
    }
    return { src: image1 || image2 };
  }, [error1, error2, image1, image2]);

  return (
    <Avatar shape="square" {...additionalAvatarProps} style={crispEdgesStyle} />
  );
}
