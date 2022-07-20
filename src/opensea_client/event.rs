use crate::opensea_client::Request;
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_aux::prelude::*;
#[derive(Deserialize, Debug)]
pub struct Asset {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub token_id: i16,
    pub permalink: String,
}
#[derive(Deserialize, Debug)]
pub struct Event {
    pub asset: Option<Asset>,
    pub event_type: String,
    pub auction_type: Option<String>,
    pub created_date: Option<String>,
    pub starting_price: Option<String>,
}
#[derive(Deserialize, Debug)]
pub struct EventsResponse {
    pub next: Option<String>,
    pub previous: Option<String>,
    pub asset_events: Vec<Event>,
}

pub struct EventsRequest {
    asset_contract_address: String,
    event_type: Option<String>,
    auction_type: Option<String>,
    occurred_before: Option<u64>,
    occurred_after: Option<u64>,
    pub cursor: Option<String>,
}

impl EventsRequest {
    pub fn new(
        asset_contract_address: String,
        event_type: Option<String>,
        auction_type: Option<String>,
        occurred_before: Option<u64>,
        occurred_after: Option<u64>,
        cursor: Option<String>,
    ) -> Self {
        EventsRequest {
            asset_contract_address: asset_contract_address,
            event_type: event_type,
            auction_type: auction_type,
            occurred_before: occurred_before,
            occurred_after: occurred_after,
            cursor: cursor,
        }
    }
    pub fn set_asset_contract_address(&mut self, new_asset_contract_address: String) {
        self.asset_contract_address = new_asset_contract_address
    }
    pub fn set_event_type(&mut self, new_event_type: String) {
        self.event_type = Some(new_event_type)
    }
    pub fn set_auction_type(&mut self, new_auction_type: String) {
        self.auction_type = Some(new_auction_type)
    }
    pub fn set_occurred_before(&mut self, new_occurred_before: u64) {
        self.occurred_before = Some(new_occurred_before)
    }
    pub fn set_occurred_after(&mut self, new_occurred_after: u64) {
        self.occurred_after = Some(new_occurred_after)
    }
    pub fn set_cursor(&mut self, new_cursor: Option<String>) {
        self.cursor = new_cursor
    }
}
impl Request for EventsRequest {
    fn build_request(&self) -> RequestBuilder {
        let mut query: Vec<(String, String)> = vec![(
            "asset_contract_address".to_string(),
            (&self.asset_contract_address).to_string(),
        )];
        if let Some(elem) = &self.event_type {
            query.push(("event_type".to_string(), elem.to_string()));
        };
        if let Some(elem) = &self.auction_type {
            query.push(("auction_type".to_string(), elem.to_string()));
        };
        if let Some(elem) = &self.occurred_before {
            query.push(("occurred_before".to_string(), elem.to_string()));
        };
        if let Some(elem) = &self.occurred_after {
            query.push(("occurred_after".to_string(), elem.to_string()));
        };
        if let Some(elem) = &self.cursor {
            query.push(("cursor".to_string(), elem.to_string()));
        };
        reqwest::Client::new()
            .get("https://api.opensea.io/api/v1/events")
            .query(&query)
    }
}
