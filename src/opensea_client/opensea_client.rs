use anyhow::anyhow;
use async_recursion::async_recursion;
use reqwest::{header::HeaderMap, RequestBuilder};
use serde::de::DeserializeOwned;
use std::{thread::sleep, time::Duration};

pub struct OpenseaClient {
    headers: HeaderMap,
}
impl OpenseaClient {
    pub fn new(k: &str) -> Self {
        let mut h = HeaderMap::new();
        h.insert("Accept", "application/json".parse().unwrap());
        h.insert("X-API-KEY", k.parse().unwrap());
        OpenseaClient { headers: h }
    }

    pub async fn request<T: Request + Sync, U: DeserializeOwned>(
        &self,
        req: &T,
    ) -> anyhow::Result<U> {
        self.try_request::<T, U>(req, None).await
    }
    #[async_recursion]
    async fn try_request<T: Request + Sync, U: DeserializeOwned>(
        &self,
        req: &T,
        nonce: Option<u8>,
    ) -> anyhow::Result<U> {
        let n: u8 = if let Some(non) = nonce { non } else { 1_u8 };
        let r_built: RequestBuilder = req.build_request().headers(self.headers.clone());

        if let Ok(res) = r_built.send().await {
            sleep(Duration::new(0, 255_000_000));
            match res.status().into() {
                200 => Ok(res.json::<U>().await?),
                429 => {
                    if n >= 5 {
                        Err(anyhow!("Too many tries for request"))
                    } else {
                        let wait = n * 3;
                        println!(
                            "Too many requests.\nNonce: {}\nWaiting {} seconds",
                            &n, &wait
                        );
                        sleep(Duration::new(wait.into(), 0));
                        self.try_request(req, Some(n + 1)).await
                    }
                }
                all_others => Err(anyhow!("Unexpected response. Code: {}", all_others)),
            }
        } else {
            Err(anyhow!("Error sending request."))
        }
    }
}
pub trait Request {
    fn build_request(&self) -> RequestBuilder;
}