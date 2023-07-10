use crate::logger::{log_error, log_info};
use actix_web::web::Data;
use std::collections::BTreeMap;
use std::fmt::Display;
use tokio::sync::RwLock;

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) enum MimeCategory {
    Application,
    Audio,
    Font,
    Image,
    Message,
    Model,
    Multipart,
    Text,
    Video,
}

impl MimeCategory {
    const BASE_URL: &str = "https://www.iana.org/assignments/media-types";
    const INITIAL_APPLICATION: &str = include_str!("../resources/application.csv");
    const INITIAL_AUDIO: &str = include_str!("../resources/audio.csv");
    const INITIAL_FONT: &str = include_str!("../resources/font.csv");
    const INITIAL_IMAGE: &str = include_str!("../resources/image.csv");
    const INITIAL_MESSAGE: &str = include_str!("../resources/message.csv");
    const INITIAL_MODEL: &str = include_str!("../resources/model.csv");
    const INITIAL_MULTIPART: &str = include_str!("../resources/multipart.csv");
    const INITIAL_TEXT: &str = include_str!("../resources/text.csv");
    const INITIAL_VIDEO: &str = include_str!("../resources/video.csv");

    pub(crate) const fn get_all_categories() -> [Self; 9] {
        [
            Self::Application,
            Self::Audio,
            Self::Font,
            Self::Image,
            Self::Message,
            Self::Model,
            Self::Multipart,
            Self::Text,
            Self::Video,
        ]
    }

    pub(crate) fn compose_update_url(&self) -> String {
        let category_string = self.to_string();

        format!("{}/{}.csv", Self::BASE_URL, category_string)
    }

    const fn initial_mime(&self) -> &str {
        match self {
            Self::Application => Self::INITIAL_APPLICATION,
            Self::Audio => Self::INITIAL_AUDIO,
            Self::Font => Self::INITIAL_FONT,
            Self::Image => Self::INITIAL_IMAGE,
            Self::Message => Self::INITIAL_MESSAGE,
            Self::Model => Self::INITIAL_MODEL,
            Self::Multipart => Self::INITIAL_MULTIPART,
            Self::Text => Self::INITIAL_TEXT,
            Self::Video => Self::INITIAL_VIDEO,
        }
    }
}

impl Display for MimeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cat_str = match self {
            Self::Application => "application",
            Self::Audio => "audio",
            Self::Font => "font",
            Self::Image => "image",
            Self::Message => "message",
            Self::Model => "model",
            Self::Multipart => "multipart",
            Self::Text => "text",
            Self::Video => "video",
        };

        write!(f, "{cat_str}")
    }
}

#[derive(serde::Deserialize)]
struct DecoderMimeRow {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Template")]
    template: Option<String>,
    #[serde(rename = "Reference")]
    reference: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct MimeInfoInner {
    hash_to_mime_map: BTreeMap<[u8; 32], String>,
    mime_to_hash_map: BTreeMap<String, [u8; 32]>,
}

impl MimeInfoInner {
    fn init_with_initial_values() -> Self {
        let categories = MimeCategory::get_all_categories();
        let mut hash_to_mime_map = BTreeMap::new();
        let mut mime_to_hash_map = BTreeMap::new();

        for category in categories {
            let category_mimes = category.initial_mime();
            let mut csv_reader = csv::ReaderBuilder::new()
                .has_headers(true)
                .terminator(csv::Terminator::CRLF)
                .from_reader(category_mimes.as_bytes());

            for row in csv_reader.deserialize::<DecoderMimeRow>() {
                if row.is_err() {
                    continue;
                }

                let decoded = row.unwrap();
                let _ = decoded.reference;
                let mime_name = decoded.name;

                if mime_name.to_lowercase().contains("deprecated") {
                    continue;
                }

                if mime_name.contains(' ') {
                    continue;
                }

                let mime_type = if let Some(mime_str) = decoded.template {
                    mime_str.to_owned()
                } else {
                    let mime_category = category.to_string();

                    format!("{mime_category}/{mime_name}")
                };
                let mime_hash: [u8; 32] = blake3::hash(mime_type.as_bytes()).into();
                hash_to_mime_map.insert(mime_hash, mime_type.clone());
                mime_to_hash_map.insert(mime_type, mime_hash);
            }
        }

        Self {
            hash_to_mime_map,
            mime_to_hash_map,
        }
    }

    fn insert(&mut self, category: MimeCategory, source: &DecoderMimeRow) {
        let mime_name = &source.name;

        if mime_name.to_lowercase().contains("deprecated") {
            return;
        }

        let mime_type = if let Some(mime_str) = &source.template {
            mime_str.to_owned()
        } else {
            let mime_category = category.to_string();
            format!("{mime_category}/{mime_name}")
        };

        let mime_hash: [u8; 32] = blake3::hash(mime_type.as_bytes()).into();
        self.hash_to_mime_map.insert(mime_hash, mime_type.clone());
        self.mime_to_hash_map.insert(mime_type, mime_hash);
    }

    fn get_all_mime_hash(&self) -> Vec<(String, [u8; 32])> {
        self.mime_to_hash_map
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    fn get_all_hash_mime(&self) -> Vec<([u8; 32], String)> {
        self.hash_to_mime_map
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    fn get_by_hash(&self, hash_str: &str) -> Option<String> {
        let hash = hex::decode(hash_str);

        if hash.is_err() {
            return None;
        }

        let hash = hash.unwrap();

        if hash.len() != 32 {
            return None;
        }

        let hash: [u8; 32] = hash.try_into().unwrap();

        self.hash_to_mime_map
            .get(&hash)
            .map(|identifier| identifier.to_owned())
    }

    fn get_hash_by_mime(&self, mime_str: &str) -> Option<String> {
        self.mime_to_hash_map
            .get(mime_str)
            .map(|hash_bytes| hex::encode(hash_bytes))
    }
}

impl Default for MimeInfoInner {
    fn default() -> Self {
        Self::init_with_initial_values()
    }
}

#[derive(Debug)]
pub(crate) struct MimeInfoShared {
    inner: RwLock<MimeInfoInner>,
}

impl MimeInfoShared {
    const USER_AGENT: &'static str = "reqwest/0.11.18";

    pub(crate) async fn update_all(&self) -> anyhow::Result<()> {
        log_info!(
            "Updating MIME list from IANA with user-agent \"{}\"...",
            Self::USER_AGENT
        );
        let categories = MimeCategory::get_all_categories();
        let http_client = reqwest::ClientBuilder::new()
            .user_agent(Self::USER_AGENT)
            .build()?;

        for category in categories {
            let category_url = category.compose_update_url();
            log_info!("MIME source: \"{category_url}\"");
            let mime_response = http_client.get(category_url).send().await?;
            let category_mimes = mime_response.text().await?;

            let mut csv_reader = csv::ReaderBuilder::new()
                .has_headers(true)
                .terminator(csv::Terminator::CRLF)
                .from_reader(category_mimes.as_bytes());

            for row in csv_reader.deserialize::<DecoderMimeRow>() {
                if row.is_err() {
                    log_error!("Failed to parse CSV for category: \"{category}\"");
                    continue;
                }

                self.insert_mime(category, &row.unwrap()).await;
            }
        }

        log_info!("MIME update finished");

        Ok(())
    }

    async fn insert_mime<'a>(&self, category: MimeCategory, source: &DecoderMimeRow) {
        let mut write_guard = self.inner.write().await;
        write_guard.insert(category, source);
    }

    pub(crate) async fn get_all_hash_to_mime(&self) -> Vec<(String, String)> {
        self.inner
            .read()
            .await
            .get_all_hash_mime()
            .into_iter()
            .map(|(hash_bytes, identifier)| (hex::encode(hash_bytes), identifier))
            .collect()
    }

    pub(crate) async fn get_all_mime_to_hash(&self) -> Vec<(String, String)> {
        self.inner
            .read()
            .await
            .get_all_mime_hash()
            .into_iter()
            .map(|(identifier, hash_bytes)| (identifier, hex::encode(hash_bytes)))
            .collect()
    }

    pub(crate) async fn get_mime_by_hash(&self, hash_str: &str) -> Option<String> {
        self.inner.read().await.get_by_hash(hash_str)
    }

    pub(crate) async fn get_hash_of_mime(&self, mime_str: &str) -> Option<String> {
        self.inner.read().await.get_hash_by_mime(mime_str)
    }
}

impl Default for MimeInfoShared {
    fn default() -> Self {
        Self {
            inner: RwLock::new(Default::default()),
        }
    }
}

impl From<MimeInfoShared> for Data<MimeInfoShared> {
    fn from(value: MimeInfoShared) -> Self {
        Data::new(value)
    }
}
