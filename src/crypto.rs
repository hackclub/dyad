use std::sync::OnceLock;

use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, KeyInit, Nonce, Payload},
};
use anyhow::{Context, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use log::warn;

const V1: &str = "v1:";
const V2: &str = "v2:";
const NONCE_LEN: usize = 12;

struct Keys {
    current: Aes256Gcm,
    previous: Option<Aes256Gcm>,
}

static KEYS: OnceLock<Keys> = OnceLock::new();

fn load(var: &str) -> anyhow::Result<Option<Aes256Gcm>> {
    let key = std::env::var(var).unwrap_or_default();
    if key.trim().is_empty() {
        return Ok(None);
    }
    let key = STANDARD
        .decode(key.trim())
        .with_context(|| format!("{var} isn't valid base64"))?;
    Aes256Gcm::new_from_slice(&key)
        .map(Some)
        .map_err(|_| anyhow!("{var} must be 32 bytes"))
}

pub fn init() -> anyhow::Result<()> {
    let current = load("TOKEN_KEY")?
        .ok_or_else(|| anyhow!("Missing variable TOKEN_KEY (32 random bytes, base64 encoded)"))?;
    let previous = load("TOKEN_KEY_PREVIOUS")?;
    let _ = KEYS.set(Keys { current, previous });
    Ok(())
}

fn keys() -> &'static Keys {
    KEYS.get().expect("crypto::init runs at startup")
}

fn aad(user_id: i32) -> Vec<u8> {
    format!("dyad:hacker:{user_id}").into_bytes()
}

pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(V1) || value.starts_with(V2)
}

pub fn has_previous_key() -> bool {
    keys().previous.is_some()
}

pub fn encrypt(user_id: i32, plaintext: &str) -> anyhow::Result<String> {
    let nonce = Nonce::<Aes256Gcm>::generate();
    let aad = aad(user_id);
    let ciphertext = keys()
        .current
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad: &aad,
            },
        )
        .map_err(|_| anyhow!("Couldn't encrypt token"))?;
    let mut bytes = nonce.to_vec();
    bytes.extend(ciphertext);
    Ok(format!("{V2}{}", STANDARD.encode(bytes)))
}

pub fn open(user_id: i32, value: &str) -> anyhow::Result<(String, bool)> {
    let (encoded, aad) = if let Some(encoded) = value.strip_prefix(V2) {
        (encoded, aad(user_id))
    } else if let Some(encoded) = value.strip_prefix(V1) {
        (encoded, Vec::new())
    } else {
        bail!("Token isn't encrypted");
    };
    let bytes = STANDARD.decode(encoded)?;
    if bytes.len() < NONCE_LEN {
        bail!("Encrypted token is too short");
    }
    let (nonce, ciphertext) = bytes.split_at(NONCE_LEN);
    let nonce = Nonce::<Aes256Gcm>::try_from(nonce)?;
    let payload = || Payload {
        msg: ciphertext,
        aad: &aad,
    };
    let keys = keys();
    let (plaintext, current) = match keys.current.decrypt(&nonce, payload()) {
        Ok(plaintext) => (plaintext, value.starts_with(V2)),
        Err(_) => keys
            .previous
            .as_ref()
            .and_then(|previous| previous.decrypt(&nonce, payload()).ok())
            .map(|plaintext| (plaintext, false))
            .ok_or_else(|| anyhow!("Couldn't decrypt token"))?,
    };
    Ok((String::from_utf8(plaintext)?, current))
}

pub fn decrypt_or_empty(user_id: i32, value: Option<String>) -> String {
    match value.as_deref() {
        None | Some("") => String::new(),
        Some(value) => open(user_id, value)
            .map(|(plaintext, _)| plaintext)
            .unwrap_or_else(|e| {
                warn!("Couldn't read a stored token: {e}");
                String::new()
            }),
    }
}
