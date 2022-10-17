use anyhow::Result;
use clap::Parser;
use rosetta_client::types::{
    AccountBalanceRequest, AccountCoinsRequest, BlockRequest, BlockTransactionRequest,
    EventsBlocksRequest, MempoolTransactionRequest, MetadataRequest, NetworkIdentifier,
    NetworkRequest,
};
use rosetta_client::{amount_to_string, Client};

mod args;
mod identifiers;

use crate::args::{AccountCommand, AccountOpts, Command, NetworkCommand, NetworkOpts, Opts};
use crate::identifiers::NetworkIdentifierOpts;

async fn network_identifier(
    client: &Client,
    opts: &NetworkIdentifierOpts,
) -> Result<NetworkIdentifier> {
    Ok(if let Some(network) = opts.network_identifier() {
        network
    } else {
        client
            .network_list(&MetadataRequest::new())
            .await?
            .network_identifiers[0]
            .clone()
    })
}

#[async_std::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let client = Client::new(&opts.url)?;

    match opts.cmd {
        Command::Network(NetworkOpts { cmd }) => match cmd {
            NetworkCommand::List => {
                let list = client.network_list(&MetadataRequest::new()).await?;
                for network in &list.network_identifiers {
                    print!("{} {}", network.blockchain, network.network);
                    if let Some(subnetwork) = network.sub_network_identifier.as_ref() {
                        print!("{}", subnetwork.network);
                    }
                    println!();
                }
            }
            NetworkCommand::Options(opts) => {
                let network = network_identifier(&client, &opts.network).await?;
                let options = client
                    .network_options(&NetworkRequest::new(network))
                    .await?;
                println!("{:#?}", options);
            }
            NetworkCommand::Status(opts) => {
                let network = network_identifier(&client, &opts.network).await?;
                let status = client.network_status(&NetworkRequest::new(network)).await?;
                println!("{:#?}", status);
            }
        },
        Command::Account(AccountOpts { cmd }) => match cmd {
            AccountCommand::Balance(opts) => {
                let req = AccountBalanceRequest {
                    network_identifier: network_identifier(&client, &opts.network).await?,
                    account_identifier: opts.account.account_identifier(),
                    block_identifier: opts.block.partial_block_identifier(),
                    currencies: None,
                };
                let balance = client.account_balance(&req).await?;
                println!(
                    "block {} {}",
                    balance.block_identifier.index, balance.block_identifier.hash
                );
                for amount in &balance.balances {
                    println!("{}", amount_to_string(amount)?);
                }
            }
            AccountCommand::Coins(opts) => {
                let req = AccountCoinsRequest {
                    network_identifier: network_identifier(&client, &opts.network).await?,
                    account_identifier: opts.account.account_identifier(),
                    currencies: None,
                    include_mempool: opts.include_mempool,
                };
                let coins = client.account_coins(&req).await?;
                println!(
                    "block {} {}",
                    coins.block_identifier.index, coins.block_identifier.hash
                );
                for coin in &coins.coins {
                    println!(
                        "{} {}",
                        coin.coin_identifier.identifier,
                        amount_to_string(&coin.amount)?
                    );
                }
            }
        },
        Command::Block(opts) => {
            let network_identifier = network_identifier(&client, &opts.network).await?;
            if let Some(transaction_identifier) = opts.transaction.transaction_identifier() {
                let block_identifier = opts
                    .block
                    .block_identifier()
                    .ok_or_else(|| anyhow::anyhow!("missing block identifier"))?;
                let req = BlockTransactionRequest {
                    network_identifier,
                    block_identifier,
                    transaction_identifier,
                };
                let res = client.block_transaction(&req).await?;
                println!("{:#?}", res);
            } else {
                let block_identifier = opts
                    .block
                    .partial_block_identifier()
                    .ok_or_else(|| anyhow::anyhow!("missing partial block identifier"))?;
                let req = BlockRequest {
                    network_identifier,
                    block_identifier,
                };
                let res = client.block(&req).await?;
                println!("{:#?}", res);
            }
        }
        Command::Mempool(opts) => {
            let network_identifier = network_identifier(&client, &opts.network).await?;
            if let Some(transaction_identifier) = opts.transaction.transaction_identifier() {
                let req = MempoolTransactionRequest {
                    network_identifier,
                    transaction_identifier,
                };
                let res = client.mempool_transaction(&req).await?;
                println!("{:#?}", res.transaction);
            } else {
                let res = client
                    .mempool(&NetworkRequest::new(network_identifier))
                    .await?;
                if res.transaction_identifiers.is_empty() {
                    println!("no pending transactions");
                }
                for transaction in &res.transaction_identifiers {
                    println!("{}", &transaction.hash);
                }
            }
        }
        Command::Events(opts) => {
            let req = EventsBlocksRequest {
                network_identifier: network_identifier(&client, &opts.network).await?,
                offset: opts.offset,
                limit: opts.limit,
            };
            let res = client.events_blocks(&req).await?;
            println!("{:#?}", res);
        }
    }
    Ok(())
}
