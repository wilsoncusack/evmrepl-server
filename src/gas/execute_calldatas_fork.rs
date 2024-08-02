use dotenv::dotenv;
use std::env;

use alloy::providers::{Provider, ProviderBuilder};
use alloy_eips::BlockId;
use alloy_primitives::{Address, Bytes, Log, U256};
use alloy_rpc_types_eth::BlockTransactionsKind;
use forge::{backend::{self, DatabaseExt}, executors::ExecutorBuilder, opts::EvmOpts, traces::CallTraceArena};
use foundry_config::Config;
use revm::{interpreter::InstructionResult, primitives::TxEnv, InnerEvmContext};
use revm_primitives::{BlockEnv, Bytecode, CfgEnv, Env};
use serde::{Deserialize, Serialize};

use crate::gas::transact;

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

    let db = executor.backend_mut();
    let mut i = InnerEvmContext::new(db);
    i.load_account(address)?;
    i.journaled_state
        .set_code(address, Bytecode::new_raw(bytecode.clone()));

    println!("YOOOO {:?}",i.journaled_state.account(address).info);
    println!("HEYYY {:?}", executor.is_empty_code(address)?);


    calls.into_iter().map(|call| {
        let r = executor.transact_raw(call.caller, address, call.calldata, call.value)?;
        Ok(ExecutionResult {
            exit_reason: r.exit_reason,
            reverted: r.reverted,
            result: r.result,
            gas_used: r.gas_used,
            logs: r.logs,
            traces: r.traces.unwrap_or(CallTraceArena::default()),
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::hex;
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
        let results = execute_calldatas_fork(bytecode, address, vec![test_call])
            .await
            .unwrap();
        println!("{:?}", results);
        // Assertions
        assert_eq!(results.len(), 1);
        let execution_result = &results[0];

        // Check that the call was successful
        assert_eq!(execution_result.exit_reason, InstructionResult::Return);
        assert!(!execution_result.reverted);

        // You might want to add more specific assertions based on what you expect from the call
        // For example, checking the return data, gas used, or logs emitted
        assert!(execution_result.gas_used > 0);
        // assert_eq!(execution_result.result, expected_result);
        // assert!(!execution_result.logs.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_simple_storage_contract() {
        // Simple storage contract bytecode
        let bytecode = Bytes::from_str("0x608060405234801561001057600080fd5b50610150806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80632e64cec11461003b5780636057361d14610059575b600080fd5b610043610075565b60405161005091906100d9565b60405180910390f35b610073600480360381019061006e919061009d565b61007e565b005b60008054905090565b8060008190555050565b60008135905061009781610103565b92915050565b6000602082840312156100b3576100b26100fe565b5b60006100c184828501610088565b91505092915050565b6100d3816100f4565b82525050565b60006020820190506100ee60008301846100ca565b92915050565b6000819050919050565b600080fd5b61010c816100f4565b811461011757600080fd5b5056fea2646970667358221220404e37f487a89a932dca5e77faaf6ca2de3b991f93d230604b1b8daaef64766264736f6c63430008070033").unwrap();
        let address = Address::from_str("0xb2f9974c62815d3177079e150377915d9bc49c82").unwrap();

        // Call to store a value
        let store_call = Call {
            caller: Address::from_str("0x1000000000000000000000000000000000000000").unwrap(),
            calldata: Bytes::from_str(
                "0x6057361d0000000000000000000000000000000000000000000000000000000000000042",
            )
            .unwrap(), // store(66)
            value: U256::from(0),
        };

        // Call to retrieve the value
        let retrieve_call = Call {
            caller: Address::from_str("0x1000000000000000000000000000000000000000").unwrap(),
            calldata: Bytes::from_str("0x2e64cec1").unwrap(), // retrieve()
            value: U256::from(0),
        };

        // Execute the calls
        let results = execute_calldatas_fork(bytecode, address, vec![store_call, retrieve_call])
            .await
            .unwrap();

        for (i, result) in results.iter().enumerate() {
            println!("Call {}", i);
            println!("Result data: 0x{}", hex::encode(&result.result));
            println!("Gas used: {}", result.gas_used);
            println!("Exit reason: {:?}", result.exit_reason);
            println!("Reverted: {}", result.reverted);
            println!("---");
        }

        // Check the retrieve call result
        assert_eq!(
            hex::encode(&results[1].result),
            "0000000000000000000000000000000000000000000000000000000000000042"
        );
    }
}
