use crate::{
    kong_data::{Marketplace, Sale, SaleType},
    opensea_client::Request,
    utils::wei_to_eth,
};
use core::fmt::Debug;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Clone)]
pub struct SeaportListing {
    pub created_date: String,
    pub closing_date: String,
    pub listing_time: u64,
    pub expiration_time: Option<u64>,
    pub current_price: String,
    pub side: String,
    pub order_type: String,
}
#[derive(Deserialize, Serialize, Clone)]
pub struct ListingsResponse {
    pub seaport_listings: Vec<SeaportListing>,
}
#[derive(Debug)]
pub struct ListingsRequest {
    pub asset_contract_address: String,
    pub token_id: i16,
    pub limit: Option<i8>,
}
/* created_timestamp: i64,
expiration_timestamp: Option<i64>,
sale_type: SaleType,
price_eth: f64,
price_usd: Option<f64>,
platform: Marketplace,
link: String, */
impl ListingsResponse {
    pub fn format_listing(&self) -> Vec<Sale> {
        self.seaport_listings
            .clone()
            .iter()
            .map(|elem| Sale {
                created_timestamp: elem.listing_time,
                expiration_timestamp: elem.expiration_time,
                sale_type: match elem.order_type.as_str() {
                    "basic" => SaleType::BuyNow,
                    _ => SaleType::Auction,
                },
                price_eth: wei_to_eth(elem.current_price.clone()),
                price_usd: None,
                platform: Marketplace::OpenSea,
            })
            .collect()
    }
}
impl ListingsRequest {
    pub fn new(asset_contract_address: String, token_id: i16, limit: Option<i8>) -> Self {
        ListingsRequest {
            asset_contract_address: asset_contract_address,
            token_id: token_id,
            limit: limit,
        }
    }
    pub fn set_token_id(&mut self, new_token_id: i16) {
        self.token_id = new_token_id
    }
}
impl Request for ListingsRequest {
    fn build_request(&self) -> RequestBuilder {
        let query_str = format!(
            "https://api.opensea.io/api/v1/asset/{}/{}/listings",
            self.asset_contract_address, self.token_id
        );
        let client = reqwest::Client::new();
        if let Some(l) = self.limit {
            client.get(query_str).query(&[("limit", l)])
        } else {
            client.get(query_str)
        }
    }
}
