import { QuestionOutlined } from '@ant-design/icons';
import { Avatar } from 'antd';
import { useMemo } from 'react';
import { z } from 'zod';
import { useImageFile } from '../../hooks/useImage';
import { useJsonWithSchema } from '../../hooks/useJsonWithSchema';

interface MercPreviewProps {
  profile: string;
}

const crispEdgesStyle = { imageRendering: 'crisp-edges' as const };

const schemaSchema = z.any();

const contentSchema = z.array(
  z.object({
    profileID: z.number(),
    internalName: z.string(),
  })
);

export function MercPreview({ profile }: MercPreviewProps) {
  const { content } = useJsonWithSchema(
    schemaSchema,
    contentSchema,
    'mercs-profile-info.json'
  );
  const profileId = useMemo(() => {
    if (!content) {
      return null;
    }
    const p = content.find((p) => p.internalName === profile);
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
