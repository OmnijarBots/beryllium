use errors::{BerylliumError, BerylliumResult};
use futures::{Future, Stream};
use futures::future;
use hyper::{Body, Error as HyperError, Method, StatusCode};
use hyper::header::{Authorization, Bearer, ContentLength};
use hyper::server::{Service, Request, Response};
use otr_manager::OtrManager;
use serde_json;
use service::BotHandler;
use std::path::PathBuf;
use std::rc::Rc;
use types::{BotCreationData, BotCreationResponse};

impl Service for BotHandler {
    type Request = Request;
    type Response = Response;
    type Error = HyperError;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let store_path = self.path();
        let mut resp = Response::new();
        let (method, uri, _version, headers, body) = req.deconstruct();

        if method != Method::Post {     // only allow POST
            resp.set_status(StatusCode::MethodNotAllowed);
            return Box::new(future::ok(resp))
        } else {        // all requests should have Bearer token auth
            match headers.get::<Authorization<Bearer>>() {
                Some(header) if self.check_auth(&header.to_string()[7..]) => (),
                _ => {
                    resp.set_status(StatusCode::Unauthorized);
                    return Box::new(future::ok(resp))
                }
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
                        let result = $call(value, store_path, &mut resp).and_then(|v| {
                            serde_json::to_vec(&v).map_err(BerylliumError::from)
                        });

                        match result {
                            Ok(bytes) => resp.set_body(Body::from(bytes)),
                            Err(e) => {
                                error!("{}", e);
                                resp.set_status(StatusCode::InternalServerError);
                            },
                        }
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
                parse_json_and!(create_bot)
            },
            _ => Box::new(future::ok(resp.with_status(StatusCode::NotFound))),
        }
    }
}

fn create_bot(data: BotCreationData, path: Rc<PathBuf>, resp: &mut Response)
              -> BerylliumResult<BotCreationResponse>
{
    info!("Creating new bot...");

    let otr = OtrManager::new(&*path, &data.id)?;
    let mut prekeys = otr.initialize(data.conversation.members.len())?;
    // There will always be a final prekey corresponding to u16::MAX
    let final_key = prekeys.pop().unwrap();

    let data = BotCreationResponse {
        prekeys: prekeys,
        last_prekey: final_key,
    };

    resp.set_status(StatusCode::Created);
    Ok(data)
}
