use futures::{Future, Stream};
use futures::future;
use hyper::{Method, Error as HyperError, StatusCode};
use hyper::header::{Authorization, Bearer, ContentLength};
use hyper::server::{Service, Request, Response};
use serde_json;
use service::BotHandler;
use std::path::PathBuf;
use std::rc::Rc;

impl Service for BotHandler {
    type Request = Request;
    type Response = Response;
    type Error = HyperError;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let store_path = self.path();
        let mut resp = Response::new();
        let (method, uri, _version, headers, body) = req.deconstruct();

        if method != Method::Post {
            resp.set_status(StatusCode::MethodNotAllowed);
        } else {
            match headers.get::<Authorization<Bearer>>() {
                Some(header) if self.check_auth(&header.to_string()[7..]) => (),
                _ => resp.set_status(StatusCode::Unauthorized)
            }
        }

        macro_rules! parse_json_and {
            ($call:expr) => {{
                let mut bytes = vec![];
                if let Some(len) = headers.get::<ContentLength>() {
                    bytes = Vec::with_capacity(**len as usize);
                }

                let f = body.fold(bytes, |mut acc, ref chunk| {
                    acc.extend_from_slice(chunk);
                    future::ok::<_, Self::Error>(acc)
                }).map(|vec| {
                    if let Ok(value) = serde_json::from_slice(&vec) {
                        $call(value, &mut resp, store_path);
                    } else {
                        resp.set_status(StatusCode::BadRequest);
                    }

                    resp
                });

                Box::new(f)
            }};
        }

        match (method, uri.path()) {
            (Method::Post, "/bots") => {
                return parse_json_and!(create_bot)
            },
            _ => resp.set_status(StatusCode::NotFound),
        }

        Box::new(future::ok(resp))
    }
}

fn create_bot(data: serde_json::Value, resp: &mut Response, path: Rc<PathBuf>) {
    resp.set_status(StatusCode::Ok)
}
