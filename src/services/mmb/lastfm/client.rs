use isahc::prelude::*;

pub struct LastFMClient {
    api_key: &'static str,
    api_secret: &'static str,
    auth_token: Option<String>,
    auth_session: Option<String>,
    endpoint: String,
    ua: &'static str,
}

impl LastFMClient {
    pub fn new(key: &'static str, secret: &'static str) -> Self {
        LastFMClient {
            api_key: key,
            api_secret: secret,
            auth_token: None,
            auth_session: None,
            endpoint: "https://ws.audioscrobbler.com/2.0/".to_string(),
            ua: "Muzak/0.1, lastfm-mmb/0.1",
        }
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        self.endpoint = endpoint;
    }

    pub fn set_session(&mut self, session: String) {
        self.auth_session = Some(session);
    }
}
