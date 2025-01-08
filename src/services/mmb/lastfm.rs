mod client;
mod requests;
mod types;
mod util;

const API_KEY: Option<&'static str> = option_env!("LASTFM_API_KEY");
const API_SECRET: Option<&'static str> = option_env!("LASTFM_API_SECRET");

pub struct LastFM {
    api_secret: String,
}

impl LastFM {
    pub fn new(api_secret: String) -> Self {
        LastFM { api_secret }
    }
}
