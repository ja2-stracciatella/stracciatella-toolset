use std::fs::OpenOptions;
use std::io::Read;

use serde::{Serialize, Deserialize};
use stracciatella::{unicode::Nfc, vfs::VfsLayer};

use crate::{
    error::{Error, Result},
    state,
};

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
pub struct ReadSoundParams {
  file: String
}

pub fn read_sound(
    state: &state::AppState,
    params: ReadSoundParams,
) -> Result<Base64Sound> {
    if params.file.contains("..") {
        return Err(Error::new("file path cannot contain `..`"));
    }

    let state = state.read();
    let selected_mod = state.try_selected_mod()?;
    let path = selected_mod.data_path(&params.file);

    let content: Vec<u8> = if path.exists() {
        let mut f = OpenOptions::new().open(&path)?;
        let mut result = vec![];

        f.read_to_end(&mut result)?;

        result
    } else {
        let mut f = selected_mod.vfs.open(&Nfc::caseless(&params.file))?;
        let mut result = vec![];

        f.read_to_end(&mut result)?;

        result
    };

    let lowercase = params.file.to_lowercase();
    if lowercase.ends_with(".wav") {
        return Ok(Base64Sound::Wav(content));
    }
    if lowercase.ends_with(".ogg") {
        return Ok(Base64Sound::Ogg(content));
    }

    return Err(Error::new("file does not seem to be a sound file"));
}
