import {
  ExclamationCircleOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
} from '@ant-design/icons';
import { Button } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSound } from '../../hooks/useSound';

export function SoundPreview({ path }: { path: string }) {
  const { loading, error: loadingError, data, refresh } = useSound(path);
  const [playError, setPlayError] = useState<Error | null>(null);
  const [audio, setAudio] = useState<HTMLAudioElement | null>(null);
  const togglePlay = useCallback(async () => {
    if (audio) {
      audio.pause();
    } else if (!loading && data) {
      const a = new Audio(data);
      a.onpause = () => setAudio(null);
      setAudio(a);
    }
  }, [audio, data, loading]);
  const icon = useMemo(() => {
    const error = playError || loadingError;
    if (error) {
      return <ExclamationCircleOutlined title={error.message} />;
    }
    if (audio) {
      return <PauseCircleOutlined />;
    }
    return <PlayCircleOutlined />;
  }, [audio, loadingError, playError]);

  useEffect(() => {
    refresh();
  }, [refresh]);
  useEffect(() => {
    if (audio) {
      audio.play().catch((e) => {
        if (e instanceof Error) {
          setPlayError(e);
        }
      });
    }
    return () => {
      audio?.pause();
    };
  }, [audio]);

  return <Button onClick={togglePlay}>{icon}</Button>;
}
