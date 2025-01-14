use isahc::prelude::*;
use serde::Deserialize;
use smallvec::SmallVec;
use tracing::debug;

pub struct LFMRequestBuilder {
    api_key: String,
    params: SmallVec<[(&'static str, String); 5]>,
    endpoint: String,
    signature: Option<String>,
    read: bool,
}

impl LFMRequestBuilder {
    pub fn new(api_key: String) -> Self {
        LFMRequestBuilder {
            api_key,
            params: SmallVec::new(),
            endpoint: "https://ws.audioscrobbler.com/2.0/".to_string(),
            signature: None,
            read: true,
        }
    }

    pub fn set_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }

    pub fn add_param(mut self, key: &'static str, value: String) -> Self {
        if self.signature.is_none() {
            self.params.push((key, value));
        } else {
            panic!("cannot add params after signing");
        }

        self
    }

    pub fn sign(mut self, secret: &str) -> Self {
        self.params.insert(0, ("api_key", self.api_key.clone()));

        if (self.read) {
            self.params.push(("format", "json".to_string()));
        }

        let params = self.params.clone();
        let mut sig = String::new();
        for (k, v) in params.iter() {
            sig.push_str(k);
            sig.push_str(v);
        }
        sig.push_str(secret);
        self.signature = Some(format!("{:x}", md5::compute(sig)));

        if (!self.read) {
            self.params.push(("format", "json".to_string()));
        }

        self
    }

    pub async fn send_request<T: for<'de> Deserialize<'de>>(self) -> anyhow::Result<T> {
        if self.read {
            self.send_read_request::<T>().await
        } else {
            self.send_write_request::<T>().await
        }
    }

    pub fn read(mut self) -> Self {
        self.read = true;
        self
    }

    pub fn write(mut self) -> Self {
        self.read = false;
        self
    }

    async fn send_read_request<T: for<'de> Deserialize<'de>>(self) -> anyhow::Result<T> {
        let mut url = self.endpoint.clone();
        url.push('?');

        for (k, v) in self.params.iter() {
            url.push_str(k);
            url.push('=');
            url.push_str(v);
            url.push('&');
        }

        url.push_str("api_sig=");
        url.push_str(
            self.signature
                .as_ref()
                .ok_or(anyhow::Error::msg("couldn't unwrap signature"))?,
        );

        let mut response = isahc::get_async(url).await?;
        let body = response.text().await?;
        debug!("{}", body);
        serde_json::from_str(&body).map_err(anyhow::Error::from)
    }

    async fn send_write_request<T: for<'de> Deserialize<'de>>(self) -> anyhow::Result<T> {
        // URL encode the parameters for the POST body
        let mut body = String::new();

        for (k, v) in self.params.iter() {
            body.push_str(k);
            body.push('=');
            body.push_str(&urlencoding::encode(v));
            body.push('&');
        }

        body.push_str("api_sig=");
        body.push_str(
            self.signature
                .as_ref()
                .ok_or(anyhow::Error::msg("couldn't unwrap signature"))?,
        );

        let mut response = isahc::post_async(self.endpoint, body).await?;
        let body = response.text().await?;
        debug!("{}", body);
        serde_json::from_str(&body).map_err(anyhow::Error::from)
    }
}
