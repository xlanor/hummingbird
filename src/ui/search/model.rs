use ahash::AHashMap;
use gpui::*;

pub struct SearchModel {
    query: String,
    results: Vec<(u32, String)>,
}
