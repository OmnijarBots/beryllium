use hyper::header::{Authorization, Bearer, Headers};

const HOST_ADDRESS: &'static str = "https://prod-nginz-https.wire.com";

#[derive(Clone)]
pub struct BotClient {
    headers: Headers,
}

impl BotClient {
    pub fn new<S: Into<String>>(token: S) -> BotClient {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer {
            token: token.into(),
        }));

        BotClient {
            headers: headers,
        }
    }
}
