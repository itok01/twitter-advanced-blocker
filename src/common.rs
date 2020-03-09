use rand::prelude::*;

use super::config::*;

// ドメインとパスを合成してURLを生成
pub fn endpoint(path: &str) -> String {
    let config = load_config();
    if config.secure {
        format!("https://{}{}", config.domain, path)
    } else {
        format!("http://{}{}", config.domain, path)
    }
}

const BASE_STR: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

// ランダムな文字列を生成
pub fn gen_random_string(size: usize) -> String {
    let mut rng = rand::thread_rng();
    String::from_utf8(
        BASE_STR
            .as_bytes()
            .choose_multiple(&mut rng, size)
            .cloned()
            .collect(),
    )
    .unwrap()
}
