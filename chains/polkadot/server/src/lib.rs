use anyhow::{Context, Result};
use parity_scale_codec::{Decode, Encode};
use rosetta_server::crypto::address::Address;
use rosetta_server::types::{BlockIdentifier, Coin};
use rosetta_server::{BlockchainClient, BlockchainConfig};
use sp_core::H256;
use sp_keyring::AccountKeyring;
use sp_runtime::{AccountId32, MultiAddress};
use subxt::metadata::DecodeStaticType;
use subxt::storage::address::{StorageHasher, StorageMapKey, Yes};
use subxt::storage::StaticStorageAddress;
use subxt::tx::{PairSigner, StaticTxPayload};
use subxt::{OnlineClient, PolkadotConfig};

pub struct PolkadotClient {
    config: BlockchainConfig,
    client: OnlineClient<PolkadotConfig>,
    genesis_block: BlockIdentifier,
}

#[async_trait::async_trait]
impl BlockchainClient for PolkadotClient {
    async fn new(network: &str, addr: &str) -> Result<Self> {
        let config = rosetta_config_polkadot::config(network)?;
        let client = OnlineClient::<PolkadotConfig>::from_url(format!("ws://{}", addr)).await?;
        let genesis = client.rpc().genesis_hash().await?;
        let genesis_block = BlockIdentifier {
            index: 0,
            hash: hex::encode(genesis.as_ref()),
        };
        Ok(Self {
            config,
            client,
            genesis_block,
        })
    }

    fn config(&self) -> &BlockchainConfig {
        &self.config
    }

    fn genesis_block(&self) -> &BlockIdentifier {
        &self.genesis_block
    }

    async fn node_version(&self) -> Result<String> {
        Ok(self.client.rpc().system_version().await?)
    }

    async fn current_block(&self) -> Result<BlockIdentifier> {
        let block = self
            .client
            .rpc()
            .block(None)
            .await?
            .context("no current block")?;
        let index = block.block.header.number as _;
        let hash = block.block.header.hash();
        Ok(BlockIdentifier {
            index,
            hash: hex::encode(hash.as_ref()),
        })
    }

    async fn balance(&self, address: &Address, block: &BlockIdentifier) -> Result<u128> {
        let address: AccountId32 = address
            .address()
            .parse()
            .map_err(|err| anyhow::anyhow!("{}", err))
            .context("invalid address")?;
        let hash = self.client.metadata().storage_hash("System", "Account")?;
        let key = StaticStorageAddress::<
            DecodeStaticType<AccountInfo<u32, AccountData>>,
            Yes,
            Yes,
            Yes,
        >::new(
            "System",
            "Account",
            vec![StorageMapKey::new(
                &address,
                StorageHasher::Blake2_128Concat,
            )],
            hash,
        );

        let block = hex::decode(&block.hash)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid block"))?;
        let account_data = self
            .client
            .storage()
            .fetch_or_default(&key, Some(H256(block)))
            .await?;
        Ok(account_data.data.free)
    }

    async fn coins(&self, _address: &Address, _block: &BlockIdentifier) -> Result<Vec<Coin>> {
        anyhow::bail!("not a utxo chain")
    }

    async fn faucet(&self, address: &Address, value: u128) -> Result<Vec<u8>> {
        let address: AccountId32 = address
            .address()
            .parse()
            .map_err(|err| anyhow::anyhow!("{}", err))
            .context("invalid address")?;
        let signer = PairSigner::<PolkadotConfig, _>::new(AccountKeyring::Alice.pair());
        let dest: MultiAddress<AccountId32, u32> = MultiAddress::Id(address);
        let hash = self.client.metadata().call_hash("Balances", "transfer")?;
        let tx = StaticTxPayload::new("Balances", "transfer", Transfer { dest, value }, hash);
        let hash = self
            .client
            .tx()
            .sign_and_submit_then_watch_default(&tx, &signer)
            .await?
            .wait_for_finalized_success()
            .await?
            .extrinsic_hash();
        Ok(hash.0.to_vec())
    }
}

#[derive(Decode, Encode, Debug)]
struct AccountInfo<Index, AccountData> {
    pub nonce: Index,
    pub consumers: Index,
    pub providers: Index,
    pub sufficients: Index,
    pub data: AccountData,
}

#[derive(Decode, Encode, Debug)]
struct AccountData {
    pub free: u128,
    pub reserved: u128,
    pub misc_frozen: u128,
    pub fee_frozen: u128,
}

#[derive(Decode, Encode, Debug)]
pub struct Transfer {
    pub dest: MultiAddress<AccountId32, u32>,
    #[codec(compact)]
    pub value: u128,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_list() -> Result<()> {
        let config = rosetta_config_polkadot::config("dev")?;
        rosetta_server::tests::network_list(config).await
    }

    #[tokio::test]
    async fn test_network_options() -> Result<()> {
        let config = rosetta_config_polkadot::config("dev")?;
        rosetta_server::tests::network_options::<PolkadotClient>(config).await
    }

    #[tokio::test]
    async fn test_network_status() -> Result<()> {
        let config = rosetta_config_polkadot::config("dev")?;
        rosetta_server::tests::network_status::<PolkadotClient>(config).await
    }
}
