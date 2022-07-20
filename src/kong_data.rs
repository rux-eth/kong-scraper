use crate::{
    opensea_client::{
        event::{EventsRequest, EventsResponse},
        listing::{ListingsRequest, ListingsResponse},
        OpenseaClient,
    },
    utils::*,
};
use hex_literal;
use progress_bar::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufReader, BufWriter},
    time::Instant,
};
use web3::transports::{Batch, Http};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Marketplace {
    OpenSea,
    LooksRare,
    X2Y2,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SaleType {
    BuyNow,
    Auction,
    Bid,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct KongTraits {
    cumulative: i16,
    shooting: i8,
    finish: i8,
    defense: i8,
    vision: i8,
    background: String,
    fur: String,
    mouth: String,
    eyes: String,
    clothes: Option<String>,
    head: Option<String>,
    head_accessory: Option<String>,
    jewellery: Option<String>,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Sale {
    pub created_timestamp: u64,
    pub expiration_timestamp: Option<u64>,
    pub sale_type: SaleType,
    pub price_eth: f64,
    pub price_usd: Option<f64>,
    pub platform: Marketplace,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct KongData {
    pub name: String,
    pub bio: Option<String>,
    pub traits: KongTraits,
    pub current_sales: Vec<Sale>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Cached {
    data: HashMap<i16, KongData>,
    prev_sales_ts: u64,
    prev_names_ts: u64,
}
impl Cached {
    pub fn default() -> anyhow::Result<Self> {
        Ok(Cached {
            data: get_defaults()?,
            prev_sales_ts: 0_u64,
            prev_names_ts: 0_u64,
        })
    }
}
pub struct ScaperBot {
    cached: Cached,
    web3: web3::Web3<Batch<Http>>,
    os_client: OpenseaClient,
}

impl ScaperBot {
    pub fn init() -> anyhow::Result<Self> {
        let node_url = env::var("INFURA_MAINNET")?;
        let os_key = env::var("INFURA_MAINNET")?;

        let c: Cached = if let Ok(cac) = restore_cache(String::from("src/utils/cache.json")) {
            cac
        } else {
            Cached::default()?
        };
        Ok(ScaperBot {
            cached: c,
            web3: get_web3(node_url.as_str()).expect("couldnt get web3. check node url"),
            os_client: OpenseaClient::new(os_key.as_str()),
        })
    }

    pub fn get_all(&self) -> &Cached {
        &self.cached
    }
    pub async fn update_all(&mut self) -> anyhow::Result<()> {
        self.update_infos().await?;
        self.update_prices().await?;
        Ok(())
    }
    pub async fn update_infos(&mut self) -> anyhow::Result<()> {
        let current_ts = get_current_ts();
        self._update_names(None).await?;
        self._update_bios(None).await?;
        self.cached.prev_names_ts = current_ts;
        self._cache_updates()?;
        Ok(())
    }
    pub async fn update_prices(&mut self) -> anyhow::Result<()> {
        let current_ts = get_current_ts();
        self._update_prices().await?;
        self.cached.prev_sales_ts = current_ts;
        self._cache_updates()?;
        Ok(())
    }
    async fn _update_prices(&mut self) -> anyhow::Result<()> {
        let start = Instant::now();
        println!("Updating prices!");
        let mut to_update: Vec<i16> = self._get_ids_to_update().await?;
        let len = &to_update.len();
        println!(
            "Got tokenIds to update.\ntotal: {}\nUpdating prices now.",
            len
        );
        if to_update.len() < 1 {
            Ok(())
        } else {
            init_progress_bar(to_update.len());
            set_progress_bar_action("Price Update", Color::Blue, Style::Bold);
            let mut listing_req: ListingsRequest =
                ListingsRequest::new(get_contract_address(), 0_i16, None);
            while to_update.len() > 0 {
                listing_req.set_token_id(to_update.remove(0));
                let res: ListingsResponse = self.os_client.request(&listing_req).await?;
                let listings = res.format_listing();
                self.cached
                    .data
                    .entry(listing_req.token_id)
                    .and_modify(|prev| prev.current_sales = listings);
                inc_progress_bar();
            }
            finalize_progress_bar();
            println!(
                "Prices updated!\nNumber of updates: {}\nTime elapsed: {} Seconds!\nAverage time per update: {}",&len,start.elapsed().as_secs(),(start.elapsed().as_secs_f64()/ ((i64::try_from(len.clone()).ok().unwrap()) as f64)));
            Ok(())
        }
    }
    async fn _get_ids_to_update(&self) -> anyhow::Result<Vec<i16>> {
        println!("Getting tokenIds to update");
        let mut ids: Vec<i16> = Vec::new();
        let mut event_req = EventsRequest::new(
            get_contract_address(),
            Some("created".to_string()),
            None,
            None,
            Some(self.cached.prev_sales_ts),
            None,
        );
        let mut calls = 0;
        loop {
            let res: EventsResponse = self
                .os_client
                .request::<EventsRequest, EventsResponse>(&event_req)
                .await?;
            calls += 1;
            if calls >= 150 {
                break;
            }
            for event in &res.asset_events {
                if let Some(ass) = &event.asset {
                    ids.push(ass.token_id);
                }
            }
            if res.asset_events.len() < 1 {
                break;
            }
            if let Some(cur) = res.next {
                event_req.set_cursor(Some(cur));
            } else {
                break;
            }
        }
        println!("Got created events. Now parsing successful events");
        event_req.set_cursor(None);
        event_req.set_event_type("successful".to_string());
        calls = 0;
        loop {
            let res: EventsResponse = self
                .os_client
                .request::<EventsRequest, EventsResponse>(&event_req)
                .await?;
            calls += 1;
            if calls >= 150 {
                break;
            }
            for event in &res.asset_events {
                if let Some(ass) = &event.asset {
                    ids.push(ass.token_id);
                }
            }
            if res.asset_events.len() < 1 {
                break;
            }
            if let Some(cur) = res.next {
                event_req.set_cursor(Some(cur));
            } else {
                break;
            }
        }
        ids.dedup();
        Ok(ids)
    }
    async fn _update_names(&mut self, token_ids: Option<Vec<i16>>) -> anyhow::Result<()> {
        let start = Instant::now();
        println!("Updating names!");
        self.web3.transport().submit_batch().await?;
        let mut ids = if let Some(i) = token_ids {
            i
        } else {
            let mut counter = 0;
            (0..10_000)
                .map(|_| {
                    counter += 1;
                    i16::try_from(counter - 1).ok().unwrap()
                })
                .collect()
        };
        ids.dedup();
        let reader = BufReader::new(File::open("src/utils/kong_naming_abi.json")?);
        let con: ethabi::Contract = serde_json::from_reader(reader)?;
        let func: &ethabi::Function = con.function("names")?;
        let get_call_req = |id: &i16| {
            let mut builder = web3::types::CallRequest::builder();
            let b: web3::types::Bytes = func
                .encode_input(vec![ethabi::Token::Uint(id.clone().into())].as_slice())
                .unwrap()
                .into();
            builder = builder.data(b);
            builder =
                builder.to(hex_literal::hex!("02afD7FD5B1C190506F538B36e7741a2F33D715d").into());
            builder.build()
        };
        for id in &ids {
            self.web3.eth().call(get_call_req(id), None);
        }
        let res = self.web3.transport().submit_batch().await?;
        for (index, elem) in res.iter().enumerate() {
            let curr_id: i16 = ids[index];
            self.cached
                .data
                .entry(curr_id)
                .and_modify(|prev| prev.name = parse_name(elem, &curr_id));
        }
        println!(
            "Names updated!\nTime elapsed: {} Seconds!",
            start.elapsed().as_secs()
        );

        Ok(())
    }
    async fn _update_bios(&mut self, token_ids: Option<Vec<i16>>) -> anyhow::Result<()> {
        let start = Instant::now();
        println!("Updating bios!");

        self.web3.transport().submit_batch().await?;
        let mut ids = if let Some(i) = token_ids {
            i
        } else {
            let mut counter = 0;
            (0..10_000)
                .map(|_| {
                    counter += 1;
                    i16::try_from(counter - 1).ok().unwrap()
                })
                .collect()
        };
        ids.dedup();
        let reader = BufReader::new(File::open("src/utils/kong_naming_abi.json")?);
        let con: ethabi::Contract = serde_json::from_reader(reader)?;
        let func: &ethabi::Function = con.function("bios")?;
        let get_call_req = |id: &i16| {
            let mut builder = web3::types::CallRequest::builder();
            let b: web3::types::Bytes = func
                .encode_input(vec![ethabi::Token::Uint(id.clone().into())].as_slice())
                .unwrap()
                .into();
            builder = builder.data(b);
            builder =
                builder.to(hex_literal::hex!("02afD7FD5B1C190506F538B36e7741a2F33D715d").into());
            builder.build()
        };
        for id in &ids {
            self.web3.eth().call(get_call_req(id), None);
        }
        let res = self.web3.transport().submit_batch().await?;
        for (index, elem) in res.iter().enumerate() {
            let curr_id: i16 = ids[index];
            self.cached
                .data
                .entry(curr_id)
                .and_modify(|prev| prev.bio = parse_bio(elem));
        }
        println!(
            "Bios updated!\nTime elapsed: {} Seconds!",
            start.elapsed().as_secs()
        );

        Ok(())
    }
    fn _cache_updates(&self) -> anyhow::Result<()> {
        let writer = BufWriter::new(File::create("src/utils/cache.json")?);
        serde_json::to_writer_pretty(writer, &self.cached)?;
        Ok(())
    }
}
