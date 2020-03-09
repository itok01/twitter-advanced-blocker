use serde::Deserialize;

// 設定の構成
#[derive(Deserialize)]
pub struct Config {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub domain: String,
    pub database_host: String,
    pub database_username: String,
    pub database_password: String,
    pub database_name: String,
}

// config.jsonから設定を読み込む
pub fn load_config() -> Config {
    let config_json =
        std::fs::read_to_string("./config.json").expect("Something went wrong reading the file");

    let config: Config = serde_json::from_str(config_json.as_str()).unwrap();
    config
}

// ドメインとパスを合成してURLを生成
pub fn endpoint(path: &str) -> String {
    let config = load_config();
    format!("http://{}{}", config.domain, path)
}
