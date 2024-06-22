use super::Call;
use alloy::providers::{Provider, ProviderBuilder};
use alloy_dyn_abi::DynSolValue;
use alloy_eips::BlockId;
use alloy_json_abi::Function;
use alloy_network::Ethereum;
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types_eth::BlockTransactionsKind;
use alloy_transport_http::{Client, Http};
use forge::{
    backend,
    executors::{self, CallResult, Executor, ExecutorBuilder, RawCallResult, TracingExecutor},
    fork::{CreateFork, MultiFork},
    inspectors::InspectorStack,
    opts::EvmOpts,
};
use foundry_config::Config;
// use forge::config::{
//     ethers_solc::{artifacts::{bytecode, Evm}, EvmVersion},
//     find_project_root_path, Config,
// };
use revm::{
    db::CacheDB,
    primitives::{Bytecode, ExecutionResult, TransactTo, TxEnv},
    InMemoryDB, Inspector,
};
use revm_primitives::{BlockEnv, CfgEnv, Env, EnvWithHandlerCfg, SpecId};
use std::{str::FromStr, sync::Arc};

pub async fn execute_calldatas_fork(
    bytecode: Bytes,
    from: Address,
    calldata: Bytes,
    value: U256,
) -> Result<RawCallResult, eyre::Error> {
    let rpc_url = "https://mainnet.base.org".parse()?;

    // Create a provider with the HTTP transport using the `reqwest` crate.
    let provider = ProviderBuilder::new().on_http(rpc_url);

    // Get latest block number.
    // let latest_block_number = provider.get_block_number().await?;
    // let latest_block = provider.get_block(BlockId::latest(), BlockTransactionKind);

    let (fork_gas_price, rpc_chain_id, block) = tokio::try_join!(
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
    let env_with_handler = EnvWithHandlerCfg::new_with_spec_id(Box::new(env), SpecId::LATEST);
    let opts = EvmOpts {
        fork_url: Some("https://mainnet.base.org".into()),
        ..Default::default()
    };
    let b = backend::Backend::spawn(opts.get_fork(&Config::default(), opts.evm_env().await?));
    let mut e = executors::Executor::new(
        b,
        env_with_handler,
        InspectorStack::default(),
        U256::from(block.header.gas_limit),
    );
    let res = e.deploy(Address::ZERO, bytecode, U256::ZERO, None)?;
    let t = e.transact_raw(from, res.address, calldata, value)?;

    Ok(t)
}

#[cfg(test)]
mod test {

    use crate::compile;

    use super::*;
    use alloy::hex::FromHex;
    use alloy_dyn_abi::DynSolType;
    use alloy_json_abi::JsonAbi;
    use alloy_primitives::{address, hex};
    use alloy_sol_types::sol;
    use alloy_sol_types::SolCall;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute() {
        let solidity_code = r#"
            pragma solidity ^0.8.0;
            contract Test {
                function test(uint256 tokenId) external returns (bytes memory) {
                    bytes memory c = abi.encodeWithSelector(bytes4(keccak256("ownerOf(uint256)")), tokenId);
                    (, bytes memory res) = address(0xcB28749c24AF4797808364D71d71539bc01E76d4).call(c);
                    return abi.encode(block.number);
                }
            }
        "#;

        sol! {
            function test(uint256 tokenId) external returns (bytes memory);
        }

        let result = compile::solidity::compile(solidity_code);
        let (json_abi, bytecode) = result.unwrap();
        let abi: JsonAbi = serde_json::from_str(&json_abi).unwrap();
        let f = abi.function("test").unwrap().first().unwrap();
        // let data = hex!("0000000000000000000000000000000000000000000000000000000000000429");
        let calldata = testCall {
            tokenId: U256::from(1065),
        }
        .abi_encode();
        // let args = DynSolType::Uint(256).abi_decode(&data).unwrap();
        // let to = address!("cB28749c24AF4797808364D71d71539bc01E76d4");
        let code = Bytes::from_hex(bytecode).expect("error getting bytes");
        let res = execute_calldatas_fork(code, Address::ZERO, calldata.into(), U256::ZERO).await;
        println!("{:?}", res.unwrap().out);
    }
}
