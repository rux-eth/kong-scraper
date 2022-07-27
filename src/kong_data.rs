use crate::{
    opensea_client::{
        event::{EventsRequest, EventsResponse},
        listing::{ListingsRequest, ListingsResponse},
        OpenseaClient,
    },
    utils::*,
};
use anyhow::anyhow;
use hex_literal;
use mongodb::{options::ClientOptions, Client, Collection};
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
    mongo_coll: Collection<MongoDoc<'static>>,
}
#[derive(Serialize, Debug, Clone)]

struct MongoDoc<'a> {
    token_id: i16,
    name: &'a String,
    bio: &'a Option<String>,
    current_price: Option<f64>,
    cumulative: i16,
    shooting: i8,
    finish: i8,
    defense: i8,
    vision: i8,
    background: &'a String,
    fur: &'a String,
    mouth: &'a String,
    eyes: &'a String,
    clothes: &'a Option<String>,
    head: &'a Option<String>,
    head_accessory: &'a Option<String>,
    jewellery: &'a Option<String>,
}
impl ScaperBot {
    pub async fn init() -> anyhow::Result<Self> {
        let node_url = env::var("INFURA_MAINNET")?;
        let os_key = env::var("OS_KEY")?;
        let mongo_pw = env::var("MONGO_PW")?;
        let mongo_un = env::var("MONGO_UN")?;
        let mongo_url = format!(
            "mongodb+srv://{}:{}@cluster0.bigvo.mongodb.net/?retryWrites=true&w=majority",
            mongo_un, mongo_pw
        );
        let client = Client::with_options(ClientOptions::parse(mongo_url).await?)?;
        let db = client.database("kong-scraper");
        let collection = db.collection::<MongoDoc>("formatted");
        let c: Cached = if let Ok(cac) = restore_cache(String::from("src/utils/cache.json")) {
            cac
        } else {
            Cached::default()?
        };
        Ok(ScaperBot {
            cached: c,
            web3: get_web3(node_url.as_str()).expect("couldnt get web3. check node url"),
            os_client: OpenseaClient::new(os_key.as_str()),
            mongo_coll: collection,
        })
    }

    pub fn get_all(&self) -> &Cached {
        &self.cached
    }
    pub async fn upload_to_db<'a>(&'a self) -> anyhow::Result<()> {
        let format_data_to_doc = |data: &'a KongData, id: &i16| MongoDoc {
            token_id: *id,
            name: &data.name,
            bio: &data.bio,
            current_price: match data.current_sales.len() {
                0 => None,
                _ => Some(data.current_sales[0].price_eth),
            },
            cumulative: data.traits.cumulative,
            shooting: data.traits.shooting,
            finish: data.traits.finish,
            defense: data.traits.defense,
            vision: data.traits.vision,
            background: &data.traits.background,
            fur: &data.traits.fur,
            mouth: &data.traits.mouth,
            eyes: &data.traits.eyes,
            clothes: &data.traits.clothes,
            head: &data.traits.head,
            head_accessory: &data.traits.head_accessory,
            jewellery: &data.traits.jewellery,
        };

        self.mongo_coll.drop(None).await?;
        let mut to_upload: Vec<MongoDoc> = Vec::new();
        for i in 0..10_000 {
            let curr_id: &i16 = &i16::try_from(i).ok().unwrap();
            to_upload.push(format_data_to_doc(
                self.cached.data.get(curr_id).unwrap(),
                curr_id,
            ));
        }
        self.mongo_coll.insert_many(&to_upload, None).await?;
        /* let mut out_vec: Vec<String> = vec!["token_id,name,bio,current_price(eth),cumulative,shooting,finish,defense,vision,background,fur,mouth,eyes,clothes,head,head_accessory,jewellery".to_string()];
        for elem in to_upload {
            out_vec.push(format!(
                "{},,,,{},{},{},{},{},{},{},{},{},{},{},{},{}",
                elem.token_id,
                elem.cumulative,
                elem.shooting,
                elem.finish,
                elem.defense,
                elem.vision,
                elem.background,
                elem.fur,
                elem.mouth,
                elem.eyes,
                if let Some(e) = elem.clothes { e } else { "" },
                if let Some(e) = elem.head { e } else { "" },
                if let Some(e) = elem.head_accessory {
                    e
                } else {
                    ""
                },
                if let Some(e) = elem.jewellery { e } else { "" }
            ))
        }
        let out_str = out_vec.join("\n");
        std::fs::write("src/static_data.csv", out_str)?; */

        Ok(())
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
        event_req.set_cursor(None);
        event_req.set_event_type("cancelled".to_string());
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
// https://us-east-1.aws.data.mongodb-api.com/app/google-blnmi/endpoint/kongdata
