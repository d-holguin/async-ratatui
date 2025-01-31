use async_ratatui_core::{Result, Tui};

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run_app().await {
        println!("application exited with error: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

pub async fn run_app() -> Result<()> {
    let mut app = Tui::new(30.0, 10.0)
        .map_err(|e| format!("Failed to initialize the terminal user interface. {}", e))?;
    app.run().await?;
    Ok(())
}
