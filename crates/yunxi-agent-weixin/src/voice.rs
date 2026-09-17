use aes::Aes128;
use aes::cipher::{BlockDecryptMut, KeyInit, block_padding::Pkcs7};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ecb::Decryptor as EcbDecryptor;
use futures_util::StreamExt;
use reqwest::{Client, Method, Url};
use std::fs;
use std::time::Duration;
use thiserror::Error;
use yunxi_agent_voice::{AudioInput, SpeechToTextProvider, VoiceRuntimeClient};

use crate::{SecretString, WeixinInboundVoice};

const WEIXIN_CDN_BASE_URL: &str = "https://novac2c.cdn.weixin.qq.com/c2c/";
const WEIXIN_CDN_HOST: &str = "novac2c.cdn.weixin.qq.com";
const WEIXIN_VOICE_ENCODE_SILK: u32 = 6;
const MAX_CDN_VOICE_BYTES: usize = 25 * 1024 * 1024;
const CDN_TIMEOUT: Duration = Duration::from_secs(90);

type Aes128EcbDecryptor = EcbDecryptor<Aes128>;

#[async_trait]
pub trait WeixinVoiceTranscriber: Send + Sync {
    async fn transcribe_voice(
        &self,
        voice: &WeixinInboundVoice,
    ) -> Result<SecretString, WeixinVoiceError>;
}

#[derive(Clone)]
pub struct WeixinVoiceBridge {
    runtime: VoiceRuntimeClient,
    cdn: Client,
}

impl WeixinVoiceBridge {
    pub fn new(runtime: VoiceRuntimeClient) -> Result<Self, WeixinVoiceError> {
        let cdn = Client::builder()
            .timeout(CDN_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("yunxi-agent-weixin/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| WeixinVoiceError::CdnClient)?;
        Ok(Self { runtime, cdn })
    }

    async fn download_encrypted_voice(
        &self,
        voice: &WeixinInboundVoice,
    ) -> Result<Vec<u8>, WeixinVoiceError> {
        let url = download_url(voice)?;
        let response = self
            .cdn
            .request(Method::GET, url)
            .send()
            .await
            .map_err(|_| WeixinVoiceError::CdnDownload)?;
        if !response.status().is_success() {
            return Err(WeixinVoiceError::CdnDownload);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_CDN_VOICE_BYTES as u64)
        {
            return Err(WeixinVoiceError::MediaTooLarge);
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| WeixinVoiceError::CdnDownload)?;
            if bytes.len().saturating_add(chunk.len()) > MAX_CDN_VOICE_BYTES {
                return Err(WeixinVoiceError::MediaTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn prepare_inbound_wav(
        &self,
        voice: &WeixinInboundVoice,
    ) -> Result<Vec<u8>, WeixinVoiceError> {
        let encrypted = self.download_encrypted_voice(voice).await?;
        let key = parse_aes_key(voice.aes_key.expose())?;
        let plaintext = decrypt_aes_ecb(&encrypted, &key)?;
        if plaintext.starts_with(b"RIFF") && plaintext.get(8..12) == Some(b"WAVE") {
            return Ok(plaintext);
        }
        if voice.encode_type != Some(WEIXIN_VOICE_ENCODE_SILK)
            && !plaintext.starts_with(b"#!SILK_V3")
            && !plaintext.starts_with(b"\x02#!SILK_V3")
        {
            return Err(WeixinVoiceError::UnsupportedEncoding);
        }
        let sample_rate = voice
            .sample_rate
            .filter(|rate| (8_000..=48_000).contains(rate))
            .unwrap_or(24_000);
        tokio::task::spawn_blocking(move || decode_silk_to_wav(&plaintext, sample_rate))
            .await
            .map_err(|_| WeixinVoiceError::SilkDecode)?
    }
}

#[async_trait]
impl WeixinVoiceTranscriber for WeixinVoiceBridge {
    async fn transcribe_voice(
        &self,
        voice: &WeixinInboundVoice,
    ) -> Result<SecretString, WeixinVoiceError> {
        let wav = self.prepare_inbound_wav(voice).await?;
        let input = AudioInput::wav(wav, self.runtime.config().max_input_bytes)
            .map_err(|_| WeixinVoiceError::InvalidWav)?;
        let result = self
            .runtime
            .transcribe(input, Some("zh"))
            .await
            .map_err(|_| WeixinVoiceError::Transcription)?;
        Ok(SecretString::new(result.text))
    }
}

#[derive(Debug, Error)]
pub enum WeixinVoiceError {
    #[error("weixin voice CDN client initialization failed")]
    CdnClient,
    #[error("weixin voice CDN URL is not allowed")]
    UnsafeCdnUrl,
    #[error("weixin voice media is too large")]
    MediaTooLarge,
    #[error("weixin voice CDN download failed")]
    CdnDownload,
    #[error("weixin voice AES key is invalid")]
    InvalidAesKey,
    #[error("weixin voice decryption failed")]
    Decryption,
    #[error("weixin voice encoding is unsupported")]
    UnsupportedEncoding,
    #[error("weixin SILK decoding failed")]
    SilkDecode,
    #[error("weixin voice WAV is invalid")]
    InvalidWav,
    #[error("weixin voice transcription failed")]
    Transcription,
}

fn download_url(voice: &WeixinInboundVoice) -> Result<Url, WeixinVoiceError> {
    if let Some(full_url) = voice.full_url.as_ref().filter(|value| !value.is_empty()) {
        return validated_cdn_url(full_url.expose());
    }
    let query = voice
        .encrypt_query_param
        .as_ref()
        .filter(|value| !value.is_empty())
        .ok_or(WeixinVoiceError::UnsafeCdnUrl)?;
    let mut url = Url::parse(WEIXIN_CDN_BASE_URL).map_err(|_| WeixinVoiceError::UnsafeCdnUrl)?;
    url.set_path("/c2c/download");
    url.query_pairs_mut()
        .append_pair("encrypted_query_param", query.expose());
    Ok(url)
}

fn validated_cdn_url(value: &str) -> Result<Url, WeixinVoiceError> {
    let url = Url::parse(value).map_err(|_| WeixinVoiceError::UnsafeCdnUrl)?;
    if url.scheme() != "https"
        || url.host_str() != Some(WEIXIN_CDN_HOST)
        || !url.path().starts_with("/c2c/")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(WeixinVoiceError::UnsafeCdnUrl);
    }
    Ok(url)
}

fn parse_aes_key(value: &str) -> Result<[u8; 16], WeixinVoiceError> {
    let decoded = BASE64
        .decode(value.as_bytes())
        .map_err(|_| WeixinVoiceError::InvalidAesKey)?;
    if decoded.len() == 16 {
        return decoded
            .try_into()
            .map_err(|_| WeixinVoiceError::InvalidAesKey);
    }
    if decoded.len() == 32 {
        let text = std::str::from_utf8(&decoded).map_err(|_| WeixinVoiceError::InvalidAesKey)?;
        return hex_decode_16(text);
    }
    Err(WeixinVoiceError::InvalidAesKey)
}

fn hex_decode_16(value: &str) -> Result<[u8; 16], WeixinVoiceError> {
    if value.len() != 32 {
        return Err(WeixinVoiceError::InvalidAesKey);
    }
    let mut output = [0u8; 16];
    for (index, byte) in output.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| WeixinVoiceError::InvalidAesKey)?;
    }
    Ok(output)
}

fn decrypt_aes_ecb(ciphertext: &[u8], key: &[u8; 16]) -> Result<Vec<u8>, WeixinVoiceError> {
    Aes128EcbDecryptor::new(key.into())
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|_| WeixinVoiceError::Decryption)
}

fn decode_silk_to_wav(data: &[u8], sample_rate: u32) -> Result<Vec<u8>, WeixinVoiceError> {
    let directory = tempfile::tempdir().map_err(|_| WeixinVoiceError::SilkDecode)?;
    let path = directory.path().join("inbound.silk");
    fs::write(&path, data).map_err(|_| WeixinVoiceError::SilkDecode)?;
    silk_decoder_rs::silk_to_wav(sample_rate, &path.to_string_lossy())
        .map_err(|_| WeixinVoiceError::SilkDecode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_raw_and_hex_encoded_aes_keys() {
        let raw = [0xabu8; 16];
        assert_eq!(parse_aes_key(&BASE64.encode(raw)).expect("raw key"), raw);
        let hex = raw
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            parse_aes_key(&BASE64.encode(hex.as_bytes())).expect("hex key"),
            raw
        );
    }

    #[test]
    fn rejects_non_weixin_cdn_urls() {
        assert!(validated_cdn_url("http://novac2c.cdn.weixin.qq.com/c2c/x").is_err());
        assert!(validated_cdn_url("https://example.com/c2c/x").is_err());
        assert!(validated_cdn_url("https://novac2c.cdn.weixin.qq.com/c2c/x").is_ok());
    }
}
