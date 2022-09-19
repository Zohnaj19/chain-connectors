use crate::identifiers::{
    AccountIdentifierOpts, BlockIdentifierOpts, NetworkIdentifierOpts, TransactionIdentifierOpts,
};
use clap::Parser;

#[derive(Parser)]
pub struct Opts {
    #[clap(long, default_value = "http://127.0.0.1:8080")]
    pub url: String,
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Parser)]
pub enum Command {
    Network(NetworkOpts),
    Account(AccountOpts),
    Block(BlockOpts),
    Mempool(MempoolOpts),
    Events(EventsOpts),
}

#[derive(Parser)]
pub struct NetworkOpts {
    #[clap(subcommand)]
    pub cmd: NetworkCommand,
}

#[derive(Parser)]
pub enum NetworkCommand {
    List,
    Options(NetworkCommandOpts),
    Status(NetworkCommandOpts),
}

#[derive(Parser)]
pub struct NetworkCommandOpts {
    #[clap(flatten)]
    pub network: NetworkIdentifierOpts,
}

#[derive(Parser)]
pub struct AccountOpts {
    #[clap(subcommand)]
    pub cmd: AccountCommand,
}

#[derive(Parser)]
pub enum AccountCommand {
    Balance(AccountBalanceCommandOpts),
    Coins(AccountCoinsCommandOpts),
}

#[derive(Parser)]
pub struct AccountBalanceCommandOpts {
    #[clap(flatten)]
    pub network: NetworkIdentifierOpts,
    #[clap(flatten)]
    pub account: AccountIdentifierOpts,
    #[clap(flatten)]
    pub block: BlockIdentifierOpts,
}

#[derive(Parser)]
pub struct AccountCoinsCommandOpts {
    #[clap(flatten)]
    pub network: NetworkIdentifierOpts,
    #[clap(flatten)]
    pub account: AccountIdentifierOpts,
    #[clap(long)]
    pub include_mempool: bool,
}

#[derive(Parser)]
pub struct BlockOpts {
    #[clap(flatten)]
    pub network: NetworkIdentifierOpts,
    #[clap(flatten)]
    pub block: BlockIdentifierOpts,
    #[clap(flatten)]
    pub transaction: TransactionIdentifierOpts,
}

#[derive(Parser)]
pub struct MempoolOpts {
    #[clap(flatten)]
    pub network: NetworkIdentifierOpts,
    #[clap(flatten)]
    pub transaction: TransactionIdentifierOpts,
}

#[derive(Parser)]
pub struct EventsOpts {
    #[clap(flatten)]
    pub network: NetworkIdentifierOpts,
    #[clap(long)]
    pub offset: Option<u64>,
    #[clap(long)]
    pub limit: Option<u64>,
}
