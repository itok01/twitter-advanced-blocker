// ユーザーのブロックリストを取得
pub async fn get_blocklist_from_twitter(token: egg_mode::Token) -> std::vec::Vec<String> {
    let blocks_ids = egg_mode::user::blocks_ids(&token);
    let blocklist: std::vec::Vec<String> = blocks_ids
        .call()
        .await
        .unwrap()
        .response
        .ids
        .iter()
        .map(|x| format!("{}", x))
        .collect();
    blocklist
}
