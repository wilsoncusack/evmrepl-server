use dotenv::dotenv;
use std::env;

use alloy::providers::{Provider, ProviderBuilder};
use alloy_eips::BlockId;
use alloy_primitives::{Address, Bytes, Log, U256};
use alloy_rpc_types_eth::BlockTransactionsKind;
use forge::{backend, executors::ExecutorBuilder, opts::EvmOpts, traces::CallTraceArena};
use foundry_config::Config;
use revm::{interpreter::InstructionResult, primitives::TxEnv};
use revm_primitives::{BlockEnv, Bytecode, CfgEnv, Env};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct Call {
    pub calldata: Bytes,
    pub value: U256,
    pub caller: Address,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    pub exit_reason: InstructionResult,
    pub reverted: bool,
    pub result: Bytes,
    pub gas_used: u64,
    pub logs: Vec<Log>,
    pub traces: CallTraceArena,
}

pub async fn execute_calldatas_fork(
    bytecode: Bytes,
    address: Address,
    calls: Vec<Call>,
) -> Result<Vec<ExecutionResult>, eyre::Error> {
    dotenv().ok();
    let rpc =
        env::var("BASE_RPC").map_err(|_| eyre::eyre!("BASE_RPC environment variable not set"))?;
    let rpc_url = rpc.parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);
    let (_fork_gas_price, rpc_chain_id, block) = tokio::try_join!(
        provider.get_gas_price(),
        provider.get_chain_id(),
        provider.get_block(BlockId::latest(), BlockTransactionsKind::Hashes)
    )?;
    let cfg = CfgEnv::default().with_chain_id(rpc_chain_id);
    let block = if let Some(block) = block {
        block
    } else {
        Err(eyre::eyre!("block not found"))?
    };
    let block_env = BlockEnv {
        number: U256::from(block.header.number.expect("block number not found")),
        timestamp: U256::from(block.header.timestamp),
        coinbase: block.header.miner,
        difficulty: block.header.difficulty,
        prevrandao: Some(block.header.mix_hash.unwrap_or_default()),
        basefee: U256::from(block.header.base_fee_per_gas.unwrap_or_default()),
        gas_limit: U256::from(block.header.gas_limit),
        ..Default::default()
    };
    let env = Env {
        cfg,
        block: block_env,
        tx: TxEnv {
            chain_id: Some(rpc_chain_id),
            gas_limit: block.header.gas_limit as u64,
            ..Default::default()
        },
        ..Default::default()
    };
    let opts = EvmOpts {
        fork_url: Some(rpc),
        ..Default::default()
    };
    let backend = backend::Backend::spawn(opts.get_fork(&Config::default(), opts.evm_env().await?));
    let mut executor = ExecutorBuilder::new()
        .inspectors(|stack| stack.trace_mode(forge::traces::TraceMode::Call).logs(true))
        .build(env, backend);

    // Set the code at the given address
    let backend = executor.backend_mut();
    let code = Bytecode::new_raw(bytecode);
    backend.insert_account_info(
        address,
        revm::primitives::AccountInfo {
            code: Some(code),
            ..Default::default()
        },
    );
    // let res = executor.deploy(Address::ZERO, bytecode, U256::ZERO, None)?;

    calls
        .into_iter()
        .map(|call| {
            let r = executor.transact_raw(call.caller, address, call.calldata, call.value)?;
            Ok(ExecutionResult {
                exit_reason: r.exit_reason,
                reverted: r.reverted,
                result: r.result,
                gas_used: r.gas_used,
                logs: r.logs,
                traces: r.traces.unwrap_or(CallTraceArena::default()),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, Bytes, U256};
    use std::str::FromStr;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute_calldatas_fork() {
        // Setup
        let bytecode = Bytes::from_str("0x608060405260043610601f5760003560e01c80635c60da1b14603157602b565b36602b576029605f565b005b6029605f565b348015603c57600080fd5b5060436097565b6040516001600160a01b03909116815260200160405180910390f35b609560917f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b60d1565b565b600060c97f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b905090565b90565b3660008037600080366000845af43d6000803e80801560ef573d6000f35b3d6000fdfea264697066735822122025084b7e87bc2c585aad5f4f716c301ab5344327e65406ebe144ae592a7fcccc64736f6c63430008110033").unwrap(); // Replace with actual bytecode
        let address = Address::from_str("0xcB28749c24AF4797808364D71d71539bc01E76d4").unwrap();

        // Create a test call
        let test_call = Call {
            caller: Address::from_str("0x1000000000000000000000000000000000000000").unwrap(),
            calldata: Bytes::from_str(
                "0x6352211e0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            value: U256::from(0),
        };

        // Execute the call
        let result = execute_calldatas_fork(bytecode, address, vec![test_call])
            .await
            .unwrap();
        println!("{:?}", result);
        // Assertions
        assert_eq!(result.len(), 1);
        let execution_result = &result[0];

        // Check that the call was successful
        assert_eq!(execution_result.exit_reason, InstructionResult::Return);
        assert!(!execution_result.reverted);

        // You might want to add more specific assertions based on what you expect from the call
        // For example, checking the return data, gas used, or logs emitted
        assert!(execution_result.gas_used > 0);
        // assert_eq!(execution_result.result, expected_result);
        // assert!(!execution_result.logs.is_empty());
    }
}
