use super::Call;
use alloy::providers::ProviderBuilder;
use alloy_dyn_abi::DynSolValue;
use alloy_eips::BlockId;
use alloy_json_abi::Function;
use alloy_network::Ethereum;
use alloy_primitives::{Address, Bytes, U256};
use alloy_transport_http::{Client, Http};
use forge::{
    backend,
    executors::{self, CallResult, Executor, TracingExecutor},
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
use revm_primitives::{CfgEnv, Env, EnvWithHandlerCfg, SpecId};
use std::{str::FromStr, sync::Arc};

pub async fn execute_calldatas_fork(
    bytecode: Bytes,
    from: Address,
    func: &Function,
    args: &[DynSolValue],
    value: U256,
) -> Result<CallResult, eyre::Error> {
    // let figment = Config::figment_with_root(find_project_root_path(None).unwrap()).merge(RpcOpts);
    // let evm_opts = figment.extract::<EvmOpts>()?;
    // let mut config = Config::try_from(figment)?.sanitized();
    // let figment = Config::figment_with_root(find_project_root_path(None).unwrap()).merge(eth.rpc);
    // let evm_opts = figment.extract::<EvmOpts>()?;

    // let (env, fork, chain) = TracingExecutor::get_fork_material(&config, evm_opts).await?;

    let env = EnvWithHandlerCfg::new_with_spec_id(Box::default(), SpecId::LATEST);
    let opts = EvmOpts {
        fork_url: Some("https://mainnet.base.org".into()),
        ..Default::default()
    };
    let b = backend::Backend::spawn(opts.get_fork(&Config::default(), opts.evm_env().await?));
    // opts.get_fork(config, env);
    let mut e = executors::Executor::new(
        b,
        env,
        InspectorStack::default(),
        U256::from_str("2000000").unwrap(),
    );
    let res = e.deploy(Address::ZERO, bytecode, U256::ZERO, None)?;
    let t = e.transact(from, res.address, func, args, value, None)?;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute() {
        let solidity_code = r#"
            pragma solidity ^0.8.0;
            contract Test {
                function test(uint256 tokenId) external returns (bytes memory) {
                    bytes memory c = abi.encodeWithSelector(bytes4(keccak256("ownerOf(uint256)")), tokenId);
                    (, bytes memory res) = address(0xcB28749c24AF4797808364D71d71539bc01E76d4).call(c);
                    return res;
                }
            }
        "#;

        let result = compile::solidity::compile(solidity_code);
        let (json_abi, bytecode) = result.unwrap();
        let abi: JsonAbi = serde_json::from_str(&json_abi).unwrap();
        let f = abi.function("test").unwrap().first().unwrap();
        let data = hex!("0000000000000000000000000000000000000000000000000000000000000429");
        let args = DynSolType::Uint(256).abi_decode(&data).unwrap();
        // let to = address!("cB28749c24AF4797808364D71d71539bc01E76d4");
        let code = Bytes::from_hex(bytecode).expect("error getting bytes");
        let res = execute_calldatas_fork(code, Address::ZERO, f, &[args], U256::ZERO).await;
        println!("{:?}", res.unwrap().raw.out);
    }
}
