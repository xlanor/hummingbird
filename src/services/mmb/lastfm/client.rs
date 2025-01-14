use isahc::prelude::*;

use super::{
    requests::LFMRequestBuilder,
    types::{GetSession, GetToken, Session},
};

pub struct LastFMClient {
    api_key: String,
    api_secret: &'static str,
    auth_session: Option<String>,
    ua: &'static str,
}

impl LastFMClient {
    pub fn new(key: String, secret: &'static str) -> Self {
        LastFMClient {
            api_key: key,
            api_secret: secret,
            auth_session: None,
            ua: "Muzak/0.1, lastfm-mmb/0.1",
        }
    }

    pub fn set_session(&mut self, session: String) {
        self.auth_session = Some(session);
    }

    pub async fn get_token(&mut self) -> anyhow::Result<String> {
        let token = LFMRequestBuilder::new(self.api_key.clone())
            .add_param("method", "auth.gettoken".to_string())
            .read()
            .sign(self.api_secret)
            .send_request::<GetToken>()
            .await?;

        Ok(token.token)
    }

    pub async fn get_session(&mut self, token: String) -> anyhow::Result<Session> {
        let session = LFMRequestBuilder::new(self.api_key.clone())
            .add_param("method", "auth.getsession".to_string())
            .add_param("token", token)
            .write()
            .sign(self.api_secret)
            .send_request::<GetSession>()
            .await?;

        Ok(session.session)
    }
}
