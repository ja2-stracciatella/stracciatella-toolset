use async_fs::OpenOptions;
use futures_lite::AsyncReadExt;
use serde::Serialize;
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

#[tauri::command]
pub async fn read_sound(
    state: tauri::State<'_, state::TauriState>,
    file: String,
) -> Result<Base64Sound> {
    if file.contains("..") {
        return Err(Error::new("file path cannot contain `..`"));
    }

    let state = state.read().await;
    let selected_mod = state.try_selected_mod()?;
    let path = selected_mod.data_path(&file);

    let content: Vec<u8> = if path.exists() {
        let mut f = OpenOptions::new().open(&path).await?;
        let mut result = vec![];

        f.read_to_end(&mut result).await?;

        result
    } else {
        let mut f = selected_mod.vfs.open(&Nfc::caseless(&file))?;
        let mut result = vec![];

        f.read_to_end(&mut result)?;

        result
    };

    let lowercase = file.to_lowercase();
    if lowercase.ends_with(".wav") {
        return Ok(Base64Sound::Wav(content));
    }
    if lowercase.ends_with(".ogg") {
        return Ok(Base64Sound::Ogg(content));
    }

    return Err(Error::new("file does not seem to be a sound file"));
}
