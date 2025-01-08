use isahc::prelude::*;
use smallvec::SmallVec;

pub struct LFMRequestBuilder {
    api_key: String,
    params: SmallVec<[(&'static str, String); 5]>,
    endpoint: String,
    signature: Option<String>,
}

impl LFMRequestBuilder {
    pub fn new(api_key: String) -> Self {
        LFMRequestBuilder {
            api_key,
            params: SmallVec::new(),
            endpoint: "https://ws.audioscrobbler.com/2.0/".to_string(),
            signature: None,
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

    pub fn sign(mut self, secret: &'static str) -> Self {
        self.params.insert(0, ("api_key", self.api_key.clone()));
        self.params.push(("format", "json".to_string()));

        let params = self.params.clone();
        let mut sig = String::new();
        for (k, v) in params.iter() {
            sig.push_str(k);
            sig.push_str(v);
        }
        sig.push_str(secret);
        self.signature = Some(format!("{:x}", md5::compute(sig)));
        self
    }

    pub fn send_read_request(self) {
        let mut url = self.endpoint.clone();
        url.push_str("?");

        for (k, v) in self.params.iter() {
            url.push_str(k);
            url.push_str("=");
            url.push_str(v);
            url.push_str("&");
        }

        url.push_str("api_sig=");
        url.push_str(self.signature.as_ref().unwrap());

        let mut response = isahc::get(url).unwrap();
        let body = response.text().unwrap();
        println!("{}", body);
    }

    pub fn send_write_request(self) {
        // URL encode the parameters for the POST body
        let mut body = String::new();

        for (k, v) in self.params.iter() {
            body.push_str(k);
            body.push_str("=");
            body.push_str(&urlencoding::encode(v));
            body.push_str("&");
        }

        body.push_str("api_sig=");
        body.push_str(self.signature.as_ref().unwrap());

        let mut response = isahc::post(self.endpoint, body).unwrap();
        let body = response.text().unwrap();
        println!("{}", body);
    }
}
