use nevermind_tauri::db::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database = Database::connect("sqlite:nevermind.db").await?;
    database.migrate().await?;
    Ok(())
}
