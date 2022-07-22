pub mod kong_data;
pub mod opensea_client;
pub mod utils;

use dotenv::dotenv;
use kong_data::ScaperBot;
use log::error;
use std::time::Duration;
use tokio::{task, time};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let mut scraper = ScaperBot::init().await?;
    let updates = task::spawn(async move {
        let mut interval = time::interval(Duration::new(300, 0));
        loop {
            interval.tick().await;
            println!("Updating Collection");
            match scraper.update_all().await {
                Ok(_) => println!("Successfully updated prices"),
                Err(err) => error!("Error updating prices.\nError: {}", err),
            };
            println!("Updating DB");
            match scraper.upload_to_db().await {
                Ok(_) => println!("Successfully uploaded to DB"),
                Err(err) => error!("Error uploading to DB.\nError: {}", err),
            };
        }
    });
    updates.await;

    Ok(())
}
/*
#[get("/")]
async fn update_prices() -> io::Result<String> {
    let mut scraper = ScaperBot::init().await.expect("msg");
    println!("Updating Prices");
    scraper.update_prices().await;

    Ok(serde_json::to_string_pretty(scraper.get_all()).expect(""))
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    dotenv().ok().unwrap();
    let _rocket = rocket::build()
        .mount("/", routes![update_prices])
        .launch()
        .await?;

    Ok(())
} */
