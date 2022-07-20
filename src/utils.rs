use crate::kong_data::{Cached, KongData, KongTraits};
use hex;
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    time::{SystemTime, UNIX_EPOCH},
};
use web3::{
    transports::{Batch, Http},
    Web3,
};
pub fn restore_cache(relative_path: String) -> anyhow::Result<Cached> {
    let reader = BufReader::new(File::open(relative_path)?);
    let res: Cached = serde_json::from_reader(reader)?;
    Ok(res)
}
pub fn get_web3(node_url: &str) -> anyhow::Result<web3::Web3<Batch<Http>>> {
    let http = Http::new(node_url)?;
    let w3 = Web3::new(Batch::new(http));
    Ok(w3)
}
pub fn get_defaults() -> anyhow::Result<HashMap<i16, KongData>> {
    let reader = BufReader::new(File::open("src/utils/metadata.json")?);
    let traits: HashMap<i16, KongTraits> = serde_json::from_reader(reader)?;
    let mut def_data: HashMap<i16, KongData> = HashMap::new();
    for i in 0..10_000 {
        let id: i16 = i16::try_from(i).ok().unwrap();
        let data = KongData {
            name: format!("Kong #{}", &id),
            bio: None,
            traits: traits.get(&id).unwrap().clone(),
            current_sales: Vec::new(),
        };
        def_data.insert(id, data);
    }
    Ok(def_data)
}

pub fn parse_name(raw: &Result<serde_json::Value, web3::Error>, id: &i16) -> String {
    let default: String = format!("Kong #{}", id);
    if let Ok(s) = raw {
        let t: String = s.as_str().unwrap().to_string();
        let mut b: String = t
            .strip_prefix("0x")
            .unwrap()
            .trim_start_matches('0')
            .trim_end_matches('0')
            .to_string();
        if b.len() % 2 == 1 {
            b = format!("{b}0");
        }
        let rand = hex::decode(&b).expect("msg: &str");
        let fin: String = String::from_utf8_lossy(&rand).to_string();
        if fin.len() > 0 {
            return fin;
        } else {
            return default;
        }
    } else {
        return default;
    }
}

pub fn parse_bio(raw: &Result<serde_json::Value, web3::Error>) -> Option<String> {
    if let Ok(s) = raw {
        let t: String = s.as_str().unwrap().to_string();
        let mut b: String = t
            .strip_prefix("0x")
            .unwrap()
            .trim_start_matches('0')
            .strip_prefix("2")
            .unwrap()
            .trim_end_matches('0')
            .trim_start_matches('0')
            .to_string();
        if b.len() % 2 == 1 {
            b = format!("{b}0");
        }
        let rand = hex::decode(&b).expect("msg: &str");
        let fin: String = String::from_utf8_lossy(&rand).to_string();
        if fin.len() > 0 {
            return Some(fin);
        } else {
            return None;
        }
    } else {
        return None;
    }
}
pub fn get_current_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("")
        .as_secs()
}
pub fn get_contract_address() -> String {
    String::from("0xEf0182dc0574cd5874494a120750FD222FdB909a")
}
pub fn wei_to_eth(wei: String) -> f64 {
    wei.parse::<f64>().unwrap() / (10_i64.pow(18)) as f64
}
