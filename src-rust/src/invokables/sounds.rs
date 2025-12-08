use crate::{invokables::Invokable, state};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, Read as _};

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
pub struct Read {
    file: String,
}

impl Invokable for Read {
    type Output = Base64Sound;

    fn name() -> &'static str {
        "sound/read"
    }

    fn validate(&self) -> Result<()> {
        if self.file.contains("..") {
            return Err(anyhow!("must not contain `..`"));
        }
        Ok(())
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
        let state = state.read();
        let mut f = BufReader::new(state.open_file(&self.file).context("failed to open file")?);
        let mut content = vec![];

        f.read_to_end(&mut content).context("failed to read file")?;

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
