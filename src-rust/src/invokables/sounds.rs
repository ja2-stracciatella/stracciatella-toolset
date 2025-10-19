use crate::{invokables::Invokable, state};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Read;
use stracciatella::{unicode::Nfc, vfs::VfsLayer};

#[derive(Debug)]
pub enum Base64Sound {
    Ogg(Vec<u8>),
    Wav(Vec<u8>),
}

impl Serialize for Base64Sound {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = match self {
            Base64Sound::Ogg(v) => v,
            Base64Sound::Wav(v) => v,
        };
        let base64_data = base64::encode(&data);
        let mime_type = match self {
            Base64Sound::Ogg(_) => "audio/ogg",
            Base64Sound::Wav(_) => "audio/wav",
        };
        serializer.serialize_str(&format!("data:{};base64,{}", mime_type, base64_data))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadSound {
    file: String,
}

impl Invokable for ReadSound {
    type Output = Base64Sound;

    fn name() -> &'static str {
        "read_sound"
    }

    fn validate(&self) -> Result<()> {
        if self.file.contains("..") {
            return Err(anyhow!("must not contain `..`"));
        }
        Ok(())
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
        let state = state.read();
        let selected_mod = state
            .try_selected_mod()
            .context("failed to get selected mod")?;
        let path = selected_mod.data_path(&self.file);

        let content: Vec<u8> = if path.exists() {
            let mut f = OpenOptions::new()
                .open(&path)
                .context("failed to open file")?;
            let mut result = vec![];

            f.read_to_end(&mut result).context("failed to read file")?;

            result
        } else {
            let mut f = selected_mod
                .vfs
                .open(&Nfc::caseless(&self.file))
                .context("failed to open file from vfs")?;
            let mut result = vec![];

            f.read_to_end(&mut result)
                .context("failed to read file from vfs")?;

            result
        };

        let lowercase = self.file.to_lowercase();
        if lowercase.ends_with(".wav") {
            return Ok(Base64Sound::Wav(content));
        }
        if lowercase.ends_with(".ogg") {
            return Ok(Base64Sound::Ogg(content));
        }

        return Err(anyhow!("file does not seem to be a sound file"));
    }
}
