import { ExclamationOutlined, QuestionOutlined } from '@ant-design/icons';
import { Image, Flex } from 'antd';
import { useEffect, useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';
import { useFileJsonDiskValue } from '../../hooks/useFileJsonDiskValue';
import { isArray, isPlainObject } from 'remeda';
import { useFileLoad } from '../../hooks/useFileLoad';

interface MercPreviewProps {
  profile: string;
}

const PROFILES_FILE = 'mercs-profile-info.json';

export function MercPreview({ profile }: MercPreviewProps) {
  const loadFile = useFileLoad();
  const profiles = useFileJsonDiskValue(PROFILES_FILE);
  const profileId = useMemo(() => {
    if (!profiles || !isArray(profiles)) return null;

    const profileValue = profiles
      .filter(isPlainObject)
      .find((it) => it.internalName === profile);
    if (
      !profileValue ||
      !('profileID' in profileValue) ||
      typeof profileValue.profileID !== 'number'
    ) {
      return null;
    }
    return profileValue.profileID;
  }, [profiles, profile]);
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
    data: image1,
    error: error1,
    refresh: refresh1,
  } = useImageFile(graphic1);
  const {
    data: image2,
    error: error2,
    refresh: refresh2,
  } = useImageFile(graphic2);
  const image = useMemo(() => {
    const error = error1 && error2 ? (error1 ?? error2) : null;
    if (profileId === null || (!image1 && !image2 && !error)) {
      return <QuestionOutlined />;
    }
    if (error) {
      return <ExclamationOutlined title={error.message} />;
    }
    return <Image preview={false} src={image1 ?? image2 ?? undefined} />;
  }, [error1, error2, image1, image2, profileId]);

  useEffect(() => {
    if (!profiles) {
      loadFile(PROFILES_FILE);
    }
  }, [loadFile, profiles]);
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
