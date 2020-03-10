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
