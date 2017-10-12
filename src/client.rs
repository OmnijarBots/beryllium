use errors::{BerylliumError, BerylliumResult};
use hyper::Method;
use hyper::header::{Authorization, Bearer, ContentType, Headers};
use reqwest::{Client, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use storage::StorageManager;
use types::{BotCreationData, Devices};

const HOST_ADDRESS: &'static str = "https://prod-nginz-https.wire.com";

#[derive(Clone)]
pub struct HttpsClient {
    bot_id: String,
    headers: Headers,
    client: Client,
}

impl HttpsClient {
    pub fn new<S: Into<String>>(id: S, token: S) -> HttpsClient {
        let mut headers = Headers::new();
        headers.set(ContentType::json());
        headers.set(Authorization(Bearer {
            token: token.into(),
        }));

        HttpsClient {
            bot_id: id.into(),
            client: Client::new(),
            headers: headers,
        }
    }

    fn request<T>(&self, method: Method, rel_url: &str,
                  data: Option<T>,
                  additional_headers: Option<Headers>)
                  -> BerylliumResult<Response>
        where T: Serialize
    {
        let url = format!("{}{}", HOST_ADDRESS, rel_url);
        let mut request = self.client.request(method, &url);
        request.headers(self.headers.clone());
        if let Some(headers) = additional_headers {
            request.headers(headers);
        }

        if let Some(object) = data {
            request.json(&object);
        }

        request.send().map_err(BerylliumError::from)
    }

    pub fn send_message<T, R>(&self, data: T, ignore_missing: bool)
                             -> BerylliumResult<(R, StatusCode)>
        where T: Serialize, R: DeserializeOwned
    {
        let url = format!("/bot/messages?ignore_missing={}", ignore_missing);
        let mut resp = self.request(Method::Post, &url, Some(data), None)?;
        let json = resp.json()?;
        Ok((json, resp.status()))
    }
}

pub struct BotData {
    pub storage: StorageManager,
    pub data: BotCreationData,
    pub client: HttpsClient,
    pub devices: Option<Devices>,
}

impl BotData {
    pub fn from_storage(bot_id: &str) -> BerylliumResult<BotData> {
        let storage = StorageManager::new(&bot_id)?;
        let store_data: BotCreationData = storage.load_state()?;
        let client = HttpsClient::new(bot_id, store_data.token.as_str());
        Ok(BotData {
            storage: storage,
            data: store_data,
            client: client,
            devices: None,
        })
    }
}

// Another client for isolating internal methods from user methods.
#[derive(Clone)]
pub struct BotClient {
    inner: HttpsClient,
}

impl From<HttpsClient> for BotClient {
    fn from(client: HttpsClient) -> BotClient {
        BotClient {
            inner: client,
        }
    }
}
