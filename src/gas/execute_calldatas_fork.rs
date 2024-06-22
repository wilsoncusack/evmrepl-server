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
use foundry_config::{ethers_solc::EvmVersion, find_project_root_path, Config};
use revm::{
    db::CacheDB,
    primitives::{Bytecode, ExecutionResult, TransactTo, TxEnv},
    Evm, InMemoryDB, Inspector,
};
use revm_primitives::{CfgEnv, Env, EnvWithHandlerCfg, SpecId};
use std::{str::FromStr, sync::Arc};

pub fn execute_calldatas_fork(
    from: Address,
    to: Address,
    func: &Function,
    args: &[DynSolValue],
    value: U256,
) -> Result<CallResult, eyre::Error> {
    let env = EnvWithHandlerCfg::new_with_spec_id(Box::default(), SpecId::LATEST);
    let (m, _) = MultiFork::new();
    let b = backend::Backend::new(
        m,
        Some(CreateFork {
            enable_caching: true,
            url: "https://mainnet.base.org".into(),
            env: Env::default(),
            evm_opts: EvmOpts::default(),
        }),
    );
    let e = executors::Executor::new(
        b,
        env,
        InspectorStack::default(),
        U256::from_str("2000000").unwrap(),
    );
    let t = e.call(from, to, func, args, value, None)?;

    Ok(t)
}

#[cfg(test)]
mod test {

    use crate::compile;

    use super::*;
    use alloy_dyn_abi::DynSolType;
    use alloy_json_abi::JsonAbi;
    use alloy_primitives::{hex, address};
    use alloy_sol_types::sol;

    #[test]
    fn test_execute() {
        let solidity_code = r#"
            pragma solidity ^0.8.0;
            contract SimpleStorage {
                uint256 public storedData;

                function set(uint256 x) public returns (uint) {
                    storedData = x;
                    return block.number;
                }

                function get() public view returns (uint256) {
                    return storedData;
                }
            }
        "#;

        sol! {
            function ownerOf(uint256 tokenId) returns (address);
        }

        let result = compile::solidity::compile(solidity_code);
        let (json_abi, bytecode) = result.unwrap();
        let abi: JsonAbi = serde_json::from_str(&json_abi).unwrap();
        let f = abi.function("set").unwrap().first().unwrap();
        // let my_type: DynSolType = "uint256".parse().unwrap();
        let data = hex!("0000000000000000000000000000000000000000000000000000000000000001");
        let args = DynSolType::Uint(256).abi_decode(&data).unwrap();
        let to = address!("cB28749c24AF4797808364D71d71539bc01E76d4");
        let res = execute_calldatas_fork(Address::ZERO, to, ownerOf, &[args], U256::ZERO);
        println!("{:?}", res.err());
    }
}