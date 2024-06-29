use alloy_sol_types::sol;
use alloy_sol_types::SolCall;
use dotenv::dotenv;
use forge::constants::CHEATCODE_ADDRESS;
use std::env;

use alloy::providers::{Provider, ProviderBuilder};
use alloy_eips::BlockId;
use alloy_primitives::{Address, Bytes, Log, U256};
use alloy_rpc_types_eth::BlockTransactionsKind;
use forge::{
    backend, executors::ExecutorBuilder, inspectors::CheatsConfig, opts::EvmOpts,
    traces::CallTraceArena,
};
use foundry_config::Config;
use revm::{interpreter::InstructionResult, primitives::TxEnv};
use revm_primitives::{address, BlockEnv, CfgEnv, Env};
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
    calls: Vec<Call>,
) -> Result<Vec<ExecutionResult>, eyre::Error> {
    dotenv().ok();
    let rpc_url = env::var("BASE_RPC")
        .map_err(|_| eyre::eyre!("BASE_RPC environment variable not set"))?
        .parse()?;
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
        fork_url: Some("https://mainnet.base.org".into()),
        ..Default::default()
    };
    let backend = backend::Backend::spawn(opts.get_fork(&Config::default(), opts.evm_env().await?));
    let mut executor = ExecutorBuilder::new()
        .inspectors(|stack| {
            stack
                .trace(true)
                .logs(true)
                .cheatcodes(CheatsConfig::default().into())
        })
        .build(env, backend);
    // let res = executor.deploy(Address::ZERO, bytecode, U256::ZERO, None)?;

    ///
    sol! {
        function etch(address target, bytes calldata newRuntimeBytecode) external;
    }
    let deploy_address = address!("1000000000000000000000000000000000000000");
    let calldata = etchCall {
        target: deploy_address,
        newRuntimeBytecode: bytecode,
    }
    .abi_encode();
    let call = Call {
        caller: Address::ZERO,
        calldata: calldata.into(),
        value: U256::ZERO,
    };
    executor.transact_raw(call.caller, CHEATCODE_ADDRESS, call.calldata, call.value)?;
    ///
    calls
        .into_iter()
        .map(|call| {
            let r =
                executor.transact_raw(call.caller, deploy_address, call.calldata, call.value)?;
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
mod test {
    use super::*;
    use crate::compile;
    use alloy::hex::FromHex;
    use alloy_sol_types::sol;
    use alloy_sol_types::SolCall;
    use revm_primitives::address;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute() {
        let solidity_code = r#"
            pragma solidity ^0.8.0;

            contract Test {
                function test(uint256 tokenId) external returns (bytes memory) {
                    bytes memory c = abi.encodeWithSelector(bytes4(keccak256("ownerOf(uint256)")), tokenId);
                    (, bytes memory res) = address(0xcB28749c24AF4797808364D71d71539bc01E76d4).staticcall(c);
                    return res;
                }
            }
        "#;

        sol! {
            function test(uint tokenId) external returns (bytes memory);
        }

        let result = compile::solidity::compile(solidity_code).unwrap();
        let bytecode = result.first().unwrap().clone().bytecode;
        let calldata = testCall {
            tokenId: U256::from(1),
        }
        .abi_encode();
        let code = Bytes::from_hex(bytecode).expect("error getting bytes");
        let res = execute_calldatas_fork(
            code,
            vec![Call {
                caller: address!("881475210E75b814D5b711090a064942b6f30605"),
                calldata: calldata.into(),
                value: U256::ZERO,
            }],
        )
        .await;
        println!("{:?}", res.unwrap().first().unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute_cheatcode() {
        let solidity_code = r#"
            pragma solidity ^0.8.0;

            contract Test {
                function test() external returns (uint) {
                    return block.number;
                }
            }
        "#;

        sol! {
            function test() external;
        }

        let result = compile::solidity::compile(solidity_code).unwrap();
        let bytecode = result.first().unwrap().clone().bytecode;
        let code = Bytes::from_hex(bytecode).expect("error getting bytes");
        let calldata = testCall {}.abi_encode();
        let res = execute_calldatas_fork(
            code,
            vec![Call {
                caller: address!("881475210E75b814D5b711090a064942b6f30605"),
                calldata: calldata.into(),
                value: U256::ZERO,
            }],
        )
        .await
        .unwrap();
        let result = res;
        // assert!(!result.reverted);
        println!("{:?}", result);
    }
}
