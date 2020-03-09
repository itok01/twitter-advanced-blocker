use super::config::*;

// データベースに接続
pub fn connect_database() -> mongodb::Database {
    // 設定を読み込む
    let config = load_config();

    // データベースサーバーに接続
    let database_uri = format!(
        "mongodb://{}:{}@{}",
        config.database_username, config.database_password, config.database_host
    );
    let client = mongodb::Client::with_uri_str(database_uri.as_str()).unwrap();

    // データベースに接続
    let database = client.database(config.database_name.as_str());
    database
}

// データベースの初期化
pub fn database_init(database: mongodb::Database) {
    // 必要なコレクション
    let necessary_collection_names: Vec<&str> = vec!["user"];

    // 存在するコレクション
    let collection_names = database.list_collection_names(None).unwrap();

    // delete_meコレクションがあれば消す
    if collection_names.contains(&"delete_me".to_string()) {
        match database.collection("delete_me").drop(None) {
            Ok(_) => println!("Deleted \"delete_me\" collection."),
            Err(e) => println!("{}", e),
        }
    }

    // 足りないコレクションを作成
    for necessary_collection_name in necessary_collection_names {
        if !collection_names.contains(&necessary_collection_name.to_string()) {
            match database
                .create_collection("user", mongodb::options::CreateCollectionOptions::default())
            {
                Ok(_) => println!("Created \"{}\" collection.", necessary_collection_name),
                Err(e) => println!("{}", e),
            }
        }
    }
}
