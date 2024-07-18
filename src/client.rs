use snarkvm::prelude::{Deserialize, Network, TestnetV0 as CurrentNetwork};
use snarkvm::prelude::Block;

use anyhow::{Context, Result};
use std::time::Duration;
use snarkvm::prelude::puzzle::Solution;


#[derive(Clone)]
pub struct AleoRpcClient {
    base_url: String,
    inner: reqwest::Client,
}

impl AleoRpcClient {
    pub fn new(base_url: &str) -> Self {
        // ex: https://vm.aleo.org/api/testnet3
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            inner: reqwest::Client::builder().timeout(Duration::from_secs(5)).build().unwrap(),
        }
    }

    pub async fn get_resource<R: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<R> {
        let resp = self.inner.get(url).send().await?;
        let status = resp.status();
        let data = resp.text().await.context("get resource to text")?;
        let resource = match status.is_success() {
            true => serde_json::from_str::<R>(&data).with_context(move || format!("serialize data to resource: {}", data))?,
            false => return Err(anyhow::anyhow!("request {} failed, status: {}, body: {}", &url, status, data)),
        };
        Ok(resource)
    }
}

impl AleoRpcClient {
    pub async fn broadcast_solution(&self, solution: Solution<CurrentNetwork>) -> Result<()> {
        let url = format!("{}/solution/broadcast", self.base_url);
        let resp = self.inner.post(&url).json(&solution).send().await?;
        let status = resp.status();
        let data = resp.text().await.context("get resource to text")?;
        match status.is_success() {
            true => Ok(()),
            false => Err(anyhow::anyhow!("request {} failed, status: {}, body: {}", &url, status, data)),
        }
    }

    pub async fn get_block(&self, block_height: u32) -> Result<Block<CurrentNetwork>> {
        let url = format!("{}/block/{}", self.base_url, block_height);
        let block = self.get_resource(&url).await?;
        Ok(block)
    }

    pub async fn get_latest_block(&self) -> Result<Block<CurrentNetwork>> {
        let url = format!("{}/block/latest", self.base_url);
        let block = self.get_resource(&url).await?;
        Ok(block)
    }
}


#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct GlobalNetworkState {
    pub block_height: u32,
    pub epoch_number: u32,
    pub epoch_hash: <CurrentNetwork as Network>::BlockHash,

    pub proof_target: u64,
}

pub async fn get_network_state(client: &AleoRpcClient) -> anyhow::Result<GlobalNetworkState> {
    let block = client.get_latest_block().await?;
    let block_height = block.height();
    let epoch_number = block_height.saturating_div(<CurrentNetwork as Network>::NUM_BLOCKS_PER_EPOCH);
    // Compute the epoch starting height (a multiple of `NUM_BLOCKS_PER_EPOCH`).
    let epoch_starting_height = epoch_number.saturating_mul(<CurrentNetwork as Network>::NUM_BLOCKS_PER_EPOCH);
    // Retrieve the epoch hash, defined as the 'previous block hash' from the epoch starting height.
    let epoch_block = client.get_block(epoch_starting_height).await?;
    Ok(GlobalNetworkState {
        block_height,
        epoch_number,
        epoch_hash: epoch_block.previous_hash(),
        proof_target: block.proof_target(),
    })
}
