use anyhow::Result;
use clap::Parser;
use futures::stream::StreamExt;
use rosetta_client::types::{AccountIdentifier, BlockTransaction, TransactionIdentifier};
use rosetta_client::EthereumExt;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Opts {
    #[clap(long)]
    pub keyfile: Option<PathBuf>,
    #[clap(long)]
    pub url: Option<String>,
    #[clap(long)]
    pub blockchain: Option<String>,
    #[clap(long)]
    pub network: Option<String>,
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Parser)]
pub enum Command {
    Pubkey,
    Account,
    Balance,
    Faucet(FaucetOpts),
    Transfer(TransferOpts),
    Transaction(TransactionOpts),
    Transactions,
    MethodCall(MethodCallOpts),
}

#[derive(Parser)]
pub struct TransferOpts {
    pub account: String,
    pub amount: String,
}

#[derive(Parser)]
pub struct FaucetOpts {
    pub amount: String,
}

#[derive(Parser)]
pub struct TransactionOpts {
    pub transaction: String,
}

#[derive(Parser)]
pub struct MethodCallOpts {
    pub contract: String,
    pub method: String,
    #[clap(value_delimiter = ' ')]
    pub params: Vec<String>,
    #[clap(long, default_value = "0")]
    pub amount: u128,
}

#[async_std::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let opts = Opts::parse();
    let wallet = rosetta_client::create_wallet(
        opts.blockchain,
        opts.network,
        opts.url,
        opts.keyfile.as_deref(),
    )
    .await?;
    match opts.cmd {
        Command::Pubkey => {
            println!("0x{}", wallet.public_key().hex_bytes);
        }
        Command::Account => {
            println!("{}", wallet.account().address);
        }
        Command::Balance => {
            let balance = wallet.balance().await?;
            println!("{}", rosetta_client::amount_to_string(&balance)?);
        }
        Command::Transfer(TransferOpts { account, amount }) => {
            let amount =
                rosetta_client::string_to_amount(&amount, wallet.config().currency_decimals)?;
            let account = AccountIdentifier {
                address: account,
                sub_account: None,
                metadata: None,
            };
            let txid = wallet.transfer(&account, amount).await?;
            println!("success: {}", txid.hash);
        }
        Command::Faucet(FaucetOpts { amount }) => match wallet.config().blockchain {
            "bitcoin" => {
                let url_str = wallet.config().node_url();
                let url_obj = match surf::Url::parse(&url_str) {
                    Ok(url) => url,
                    Err(e) => {
                        anyhow::bail!("Url parse error: {}", e);
                    }
                };
                let url = match url_obj.host() {
                    Some(url) => url,
                    None => {
                        anyhow::bail!("Invalid Url");
                    }
                };

                use std::process::Command;
                let status = Command::new("bitcoin-cli")
                    .arg("-regtest")
                    .arg(format!("-rpcconnect={url}"))
                    .arg("-rpcuser=rosetta")
                    .arg("-rpcpassword=rosetta")
                    .arg("generatetoaddress")
                    .arg(amount)
                    .arg(&wallet.account().address)
                    .status()?;
                if !status.success() {
                    anyhow::bail!("cmd failed");
                }
            }
            _ => {
                let amount =
                    rosetta_client::string_to_amount(&amount, wallet.config().currency_decimals)?;
                let txid = wallet.faucet(amount).await?;
                println!("success: {}", txid.hash);
            }
        },
        Command::Transaction(TransactionOpts { transaction }) => {
            let txid = TransactionIdentifier { hash: transaction };
            let tx = wallet.transaction(txid).await?;
            print_transaction_header();
            print_transaction(&tx)?;
        }
        Command::Transactions => {
            let mut first = true;
            let mut stream = wallet.transactions(100);
            while let Some(res) = stream.next().await {
                let transactions = res?;
                if first {
                    print_transaction_header();
                    first = false;
                }
                for tx in transactions {
                    print_transaction(&tx)?;
                }
            }
            if first {
                println!("No transactions found");
            }
        }
        Command::MethodCall(MethodCallOpts {
            contract,
            method,
            params,
            amount,
        }) => {
            let tx = wallet
                .eth_send_call(&contract, &method, &params, amount)
                .await?;
            println!("Transaction hash: {:?}", tx.hash);
        }
    }
    Ok(())
}

fn print_transaction_header() {
    println!(
        "{: <8} | {: <40} | {: <25} | {: <50}",
        "Block", "Op", "Amount", "Account"
    );
}

fn print_transaction(tx: &BlockTransaction) -> Result<()> {
    let block = tx.block_identifier.index;
    for op in &tx.transaction.operations {
        let name = &op.r#type;
        let amount = op
            .amount
            .as_ref()
            .map(rosetta_client::amount_to_string)
            .transpose()?
            .unwrap_or_default();
        let account = op
            .account
            .as_ref()
            .map(|account| account.address.as_str())
            .unwrap_or_default();
        println!(
            "{: <8} | {: <40} | {: >25} | {: <50}",
            block, name, amount, account
        );
    }
    Ok(())
}
