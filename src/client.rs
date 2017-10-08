use errors::{BerylliumError, BerylliumResult};
use hyper::Method;
use hyper::header::{Authorization, Bearer, ContentType, Headers};
use reqwest::{Client, Response};
use serde::Serialize;

const HOST_ADDRESS: &'static str = "https://prod-nginz-https.wire.com";

#[derive(Clone)]
pub struct BotClient {
    bot_id: String,
    headers: Headers,
    client: Client,
}

impl BotClient {
    pub fn new<S: Into<String>>(id: S, token: S) -> BotClient {
        let mut headers = Headers::new();
        headers.set(ContentType::json());
        headers.set(Authorization(Bearer {
            token: token.into(),
        }));

        BotClient {
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
}
