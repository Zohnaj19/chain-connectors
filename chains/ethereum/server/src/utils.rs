use crate::eth_types::{
    FlattenTrace, Trace, BYZANTIUM_BLOCK_REWARD, CALL_OP_TYPE, CONSTANTINOPLE_BLOCK_REWARD,
    CREATE2_OP_TYPE, CREATE_OP_TYPE, DESTRUCT_OP_TYPE, FAILURE_STATUS, FEE_OP_TYPE,
    FRONTIER_BLOCK_REWARD, MAX_UNCLE_DEPTH, MINING_REWARD_OP_TYPE, SELF_DESTRUCT_OP_TYPE,
    SUCCESS_STATUS, TESTNET_CHAIN_CONFIG, UNCLE_REWARD_MULTIPLIER, UNCLE_REWARD_OP_TYPE,
};
use anyhow::{bail, Context, Result};
use ethers::{prelude::*, utils::to_checksum};
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Block, Transaction, TransactionReceipt, H160, H256, U256, U64},
};
use rosetta_server::types as rosetta_types;
use rosetta_server::types::{
    AccountIdentifier, Amount, Currency, Operation, OperationIdentifier, TransactionIdentifier,
};
use rosetta_server::BlockchainConfig;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;

pub async fn get_transaction<T>(
    client: &Provider<Http>,
    config: &BlockchainConfig,
    block: &Block<T>,
    tx: &Transaction,
) -> Result<rosetta_types::Transaction> {
    let tx_receipt = client
        .get_transaction_receipt(tx.hash)
        .await?
        .context("Transaction receipt not found")?;

    if tx_receipt
        .block_hash
        .context("Block hash not found in tx receipt")?
        != block.hash.unwrap()
    {
        bail!("Transaction receipt block hash does not match block hash");
    }

    let currency = config.currency();

    let mut operations = vec![];
    let fee_ops = get_fee_operations(block, tx, &tx_receipt, &currency)?;
    operations.extend(fee_ops);

    let tx_trace = if block.number.unwrap().as_u64() != 0 {
        let trace = get_transaction_trace(&tx.hash, client).await?;
        let trace_ops = get_trace_operations(trace.clone(), operations.len() as i64, &currency)?;
        operations.extend(trace_ops);
        Some(trace)
    } else {
        None
    };

    Ok(rosetta_types::Transaction {
        transaction_identifier: TransactionIdentifier {
            hash: hex::encode(tx.hash),
        },
        operations,
        related_transactions: None,
        metadata: Some(json!({
            "gas_limit" : tx.gas,
            "gas_price": tx.gas_price,
            "receipt": tx_receipt,
            "trace": tx_trace,
        })),
    })
}

fn get_fee_operations<T>(
    block: &Block<T>,
    tx: &Transaction,
    receipt: &TransactionReceipt,
    currency: &Currency,
) -> Result<Vec<Operation>> {
    let miner = block.author.context("block has no author")?;
    let base_fee = block.base_fee_per_gas.context("block has no base fee")?;
    let tx_type = tx
        .transaction_type
        .context("transaction type unavailable")?;
    let tx_gas_price = tx.gas_price.context("gas price is not available")?;
    let tx_max_priority_fee_per_gas = tx.max_priority_fee_per_gas.unwrap_or_default();
    let gas_used = receipt.gas_used.context("gas used is not available")?;
    let gas_price = if tx_type.as_u64() == 2 {
        base_fee + tx_max_priority_fee_per_gas
    } else {
        tx_gas_price
    };
    let fee_amount = gas_used * gas_price;
    let fee_burned = gas_used * base_fee;
    let miner_earned_reward = fee_amount - fee_burned;

    let mut operations = vec![];

    let first_op = Operation {
        operation_identifier: OperationIdentifier {
            index: 0,
            network_index: None,
        },
        related_operations: None,
        r#type: FEE_OP_TYPE.into(),
        status: Some(SUCCESS_STATUS.into()),
        account: Some(AccountIdentifier {
            address: to_checksum(&tx.from, None),
            sub_account: None,
            metadata: None,
        }),
        amount: Some(Amount {
            value: format!("-{miner_earned_reward}"),
            currency: currency.clone(),
            metadata: None,
        }),
        coin_change: None,
        metadata: None,
    };

    let second_op = Operation {
        operation_identifier: OperationIdentifier {
            index: 1,
            network_index: None,
        },
        related_operations: Some(vec![OperationIdentifier {
            index: 0,
            network_index: None,
        }]),
        r#type: FEE_OP_TYPE.into(),
        status: Some(SUCCESS_STATUS.into()),
        account: Some(AccountIdentifier {
            address: to_checksum(&miner, None),
            sub_account: None,
            metadata: None,
        }),
        amount: Some(Amount {
            value: format!("{miner_earned_reward}"),
            currency: currency.clone(),
            metadata: None,
        }),
        coin_change: None,
        metadata: None,
    };

    operations.push(first_op);
    operations.push(second_op);

    if fee_burned != U256::from(0) {
        let burned_operation = Operation {
            operation_identifier: OperationIdentifier {
                index: 2,
                network_index: None,
            },
            related_operations: None,
            r#type: FEE_OP_TYPE.into(),
            status: Some(SUCCESS_STATUS.into()),
            account: Some(AccountIdentifier {
                address: to_checksum(&tx.from, None),
                sub_account: None,
                metadata: None,
            }),
            amount: Some(Amount {
                value: format!("-{fee_burned}"),
                currency: currency.clone(),
                metadata: None,
            }),
            coin_change: None,
            metadata: None,
        };

        operations.push(burned_operation);
    }
    Ok(operations)
}

async fn get_transaction_trace(hash: &H256, client: &Provider<Http>) -> Result<Trace> {
    let params = json!([
        hash,
        {
            "tracer": "callTracer"
        }
    ]);
    Ok(client.request("debug_traceTransaction", params).await?)
}

fn get_trace_operations(trace: Trace, op_len: i64, currency: &Currency) -> Result<Vec<Operation>> {
    let mut traces = VecDeque::new();
    traces.push_back(trace);
    let mut flatten_traces = vec![];
    while let Some(mut trace) = traces.pop_front() {
        for mut child in std::mem::take(&mut trace.calls) {
            if trace.revert {
                child.revert = true;
                if child.error_message.is_empty() {
                    child.error_message = trace.error_message.clone();
                }
            }
            traces.push_back(child);
        }
        flatten_traces.push(FlattenTrace::from(trace));
    }
    let traces = flatten_traces;

    let mut operations: Vec<Operation> = vec![];
    let mut destroyed_accs: HashMap<String, u64> = HashMap::new();

    if traces.is_empty() {
        return Ok(operations);
    }

    for trace in traces {
        let mut metadata: HashMap<String, String> = HashMap::new();
        let mut operation_status = SUCCESS_STATUS;
        if trace.revert {
            operation_status = FAILURE_STATUS;
            metadata.insert("error".into(), trace.error_message);
        }

        let mut zero_value = false;
        if trace.value == U256::from(0) {
            zero_value = true;
        }

        let mut should_add = true;
        if zero_value && trace.trace_type == CALL_OP_TYPE {
            should_add = false;
        }

        let from = to_checksum(&trace.from, None);
        let to = to_checksum(&trace.to, None);

        if should_add {
            let mut from_operation = Operation {
                operation_identifier: OperationIdentifier {
                    index: op_len + operations.len() as i64,
                    network_index: None,
                },
                related_operations: None,
                r#type: trace.trace_type.clone(),
                status: Some(operation_status.into()),
                account: Some(AccountIdentifier {
                    address: from.clone(),
                    sub_account: None,
                    metadata: None,
                }),
                amount: Some(Amount {
                    value: format!("-{}", trace.value),
                    currency: currency.clone(),
                    metadata: None,
                }),
                coin_change: None,
                metadata: None,
            };

            if zero_value {
                from_operation.amount = None;
            } else if let Some(d_from) = destroyed_accs.get(&from) {
                if operation_status == SUCCESS_STATUS {
                    let amount = d_from - trace.value.as_u64();
                    destroyed_accs.insert(from.clone(), amount);
                }
            }

            operations.push(from_operation);
        }

        if trace.trace_type == SELF_DESTRUCT_OP_TYPE {
            //assigning destroyed from to an empty number
            if from == to {
                continue;
            }
        }

        if to.is_empty() {
            continue;
        }

        // If the account is resurrected, we remove it from
        // the destroyed accounts map.
        if trace.trace_type == CREATE_OP_TYPE || trace.trace_type == CREATE2_OP_TYPE {
            destroyed_accs.remove(&to);
        }

        if should_add {
            let last_op_index = operations[operations.len() - 1].operation_identifier.index;
            let mut to_op = Operation {
                operation_identifier: OperationIdentifier {
                    index: last_op_index + 1,
                    network_index: None,
                },
                related_operations: Some(vec![OperationIdentifier {
                    index: last_op_index,
                    network_index: None,
                }]),
                r#type: trace.trace_type,
                status: Some(operation_status.into()),
                account: Some(AccountIdentifier {
                    address: to.clone(),
                    sub_account: None,
                    metadata: None,
                }),
                amount: Some(Amount {
                    value: format!("{}", trace.value),
                    currency: currency.clone(),
                    metadata: None,
                }),
                coin_change: None,
                metadata: None,
            };

            if zero_value {
                to_op.amount = None;
            } else if let Some(d_to) = destroyed_accs.get(&to) {
                if operation_status == SUCCESS_STATUS {
                    let amount = d_to + trace.value.as_u64();
                    destroyed_accs.insert(to.clone(), amount);
                }
            }

            operations.push(to_op);
        }

        for (k, v) in &destroyed_accs {
            if v == &0 {
                continue;
            }

            if v < &0 {
                //throw some error
            }

            let operation = Operation {
                operation_identifier: OperationIdentifier {
                    index: operations[operations.len() - 1].operation_identifier.index + 1,
                    network_index: None,
                },
                related_operations: None,
                r#type: DESTRUCT_OP_TYPE.into(),
                status: Some(SUCCESS_STATUS.into()),
                account: Some(AccountIdentifier {
                    address: to_checksum(&H160::from_str(k)?, None),
                    sub_account: None,
                    metadata: None,
                }),
                amount: Some(Amount {
                    value: format!("-{v}"),
                    currency: currency.clone(),
                    metadata: None,
                }),
                coin_change: None,
                metadata: None,
            };

            operations.push(operation);
        }
    }

    Ok(operations)
}

pub async fn block_reward_transaction(
    client: &Provider<Http>,
    config: &BlockchainConfig,
    block: &Block<Transaction>,
) -> Result<rosetta_types::Transaction> {
    let block_number = block.number.context("missing block number")?.as_u64();
    let block_hash = block.hash.context("missing block hash")?;
    let block_id = BlockId::Hash(block_hash);
    let miner = block.author.unwrap();

    let mut uncles = vec![];
    for (i, _) in block.uncles.iter().enumerate() {
        let uncle = client
            .get_uncle(block_id, U64::from(i))
            .await?
            .context("Uncle block now found")?;
        uncles.push(uncle);
    }

    let chain_config = TESTNET_CHAIN_CONFIG;
    let mut mining_reward = FRONTIER_BLOCK_REWARD;
    if chain_config.byzantium_block <= block_number {
        mining_reward = BYZANTIUM_BLOCK_REWARD;
    }
    if chain_config.constantinople_block <= block_number {
        mining_reward = CONSTANTINOPLE_BLOCK_REWARD;
    }
    if !uncles.is_empty() {
        mining_reward += (mining_reward / UNCLE_REWARD_MULTIPLIER) * mining_reward;
    }

    let mut operations = vec![];
    let mining_reward_operation = Operation {
        operation_identifier: OperationIdentifier {
            index: 0,
            network_index: None,
        },
        related_operations: None,
        r#type: MINING_REWARD_OP_TYPE.into(),
        status: Some(SUCCESS_STATUS.into()),
        account: Some(AccountIdentifier {
            address: to_checksum(&miner, None),
            sub_account: None,
            metadata: None,
        }),
        amount: Some(Amount {
            value: mining_reward.to_string(),
            currency: config.currency(),
            metadata: None,
        }),
        coin_change: None,
        metadata: None,
    };
    operations.push(mining_reward_operation);

    for block in uncles {
        let uncle_miner = block.author.context("Uncle block has no author")?;
        let uncle_number = block.number.context("Uncle block has no number")?;
        let uncle_block_reward =
            (uncle_number + MAX_UNCLE_DEPTH - block_number) * (mining_reward / MAX_UNCLE_DEPTH);

        let operation = Operation {
            operation_identifier: OperationIdentifier {
                index: operations.len() as i64,
                network_index: None,
            },
            related_operations: None,
            r#type: UNCLE_REWARD_OP_TYPE.into(),
            status: Some(SUCCESS_STATUS.into()),
            account: Some(AccountIdentifier {
                address: to_checksum(&uncle_miner, None),
                sub_account: None,
                metadata: None,
            }),
            amount: Some(Amount {
                value: uncle_block_reward.to_string(),
                currency: config.currency(),
                metadata: None,
            }),
            coin_change: None,
            metadata: None,
        };
        operations.push(operation);
    }

    Ok(rosetta_types::Transaction {
        transaction_identifier: TransactionIdentifier {
            hash: hex::encode(block_hash),
        },
        related_transactions: None,
        operations,
        metadata: None,
    })
}
