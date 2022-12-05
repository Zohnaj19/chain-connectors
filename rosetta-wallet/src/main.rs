use anyhow::Result;
use clap::Parser;
use rosetta_client::types::AccountIdentifier;
use rosetta_client::Chain;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Opts {
    #[clap(long)]
    pub keyfile: Option<PathBuf>,
    #[clap(long)]
    pub url: Option<String>,
    #[clap(long)]
    pub chain: Chain,
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Parser)]
pub enum Command {
    Pubkey,
    Account,
    Balance,
    Transfer(TransferOpts),
    Faucet(FaucetOpts),
}

#[derive(Parser)]
pub struct TransferOpts {
    pub account: String,
    pub amount: u128,
}

#[derive(Parser)]
pub struct FaucetOpts {
    pub amount: u128,
}

#[async_std::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let opts = Opts::parse();
    let wallet =
        rosetta_client::create_wallet(opts.chain, opts.url.as_deref(), opts.keyfile.as_deref())?;

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
            let account = AccountIdentifier {
                address: account,
                sub_account: None,
                metadata: None,
            };
            let txid = wallet.transfer(&account, amount).await?;
            println!("{}", txid.hash);
        }
        Command::Faucet(FaucetOpts { amount }) => match opts.chain {
            Chain::Btc => {
                use std::process::Command;
                let status = Command::new("bitcoin-cli")
                    .arg("-regtest")
                    .arg("-rpcconnect=rosetta.analog.one")
                    .arg("-rpcuser=rosetta")
                    .arg("-rpcpassword=rosetta")
                    .arg("generatetoaddress")
                    .arg(amount.to_string())
                    .arg(&wallet.account().address)
                    .status()?;
                if !status.success() {
                    anyhow::bail!("cmd failed");
                }
            }
            Chain::Eth => {
                use std::process::Command;
                let status = Command::new("geth")
                    .arg("attach")
                    .arg("--exec")
                    .arg(format!(
                        "eth.sendTransaction({{from: eth.coinbase, to: '{}', value: {}}})",
                        &wallet.account().address,
                        amount,
                    ))
                    .arg("http://rosetta.analog.one:8545")
                    .status()?;
                if !status.success() {
                    anyhow::bail!("cmd failed");
                }
            }
            Chain::Dot => {
                match wallet.faucet_dev(amount).await {
                    Ok(data) => {
                        println!("success: {}", data.hash);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        return Ok(());
                    }
                };
            }
        },
    }
    Ok(())
}
