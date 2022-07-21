import { PauseCircleOutlined, PlayCircleOutlined } from "@ant-design/icons";
import { Button } from "antd";
import { useCallback, useEffect, useState } from "react";
import { readSound } from "../../lib/readSound";

export function SoundPreview({ path }: { path: string }) {
  const [audio, setAudio] = useState<HTMLAudioElement | null>(null);
  const togglePlay = useCallback(async () => {
    if (!audio) {
      // load audio
      const src = await readSound(path);
      const audio = new Audio(src);

      audio.onpause = () => setAudio(null);

      setAudio(audio);
    } else {
      if (audio) {
        audio.pause();
      }
    }
  }, [audio, path]);

  useEffect(() => {
    if (audio) {
      audio.play();
    }
  }, [audio, path]);

  return (
    <Button onClick={togglePlay}>
      {audio ? <PauseCircleOutlined /> : <PlayCircleOutlined />}
    </Button>
  );
}
