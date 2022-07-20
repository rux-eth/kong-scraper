#[macro_use]
extern crate rocket;
pub mod kong_data;
pub mod opensea_client;
pub mod utils;

use dotenv::dotenv;
use kong_data::ScaperBot;

use std::io;

#[get("/")]
async fn update_prices() -> io::Result<String> {
    let mut scraper = ScaperBot::init().expect("msg");
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
}
