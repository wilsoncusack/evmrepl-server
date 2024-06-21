use revm::{
    db::{AlloyDB, CacheDB}, primitives::{Bytecode, ExecutionResult, TransactTo, TxEnv}, Evm, InMemoryDB
};
use alloy::providers::{ProviderBuilder, };
use alloy_eips::BlockId;
use alloy_network::{Ethereum};
use alloy_transport_http::{Client, Http};
use std::sync::Arc;

use super::{Call};

pub fn execute_calldatas_fork(
    bytecode: Bytecode,
    calls: Vec<Call>,
) -> Result<Vec<ExecutionResult>, eyre::Error> {
    let rpc_url = "https://mainnet.base.org".parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);
    let client = Arc::new(provider);
    let mut alloydb: AlloyDB<Http<Client>, Ethereum, Arc<alloy::providers::RootProvider<Http<Client>>>>  = AlloyDB::new(Arc::clone(&client), BlockId::latest());
    //AlloyDB<Http<Client>, Ethereum, Client>
    
    // alloydb.

    let mut db = CacheDB::new(alloydb);
    // alloydb.db.
    // db.insert_account_info(pool_address, acc_info);

    let address = deploy(bytecode.bytes(), &mut db)?;

    calls
        .into_iter()
        .map(|call| transact_fork(address, call.calldata, call.value, call.caller, &mut db))
        .collect()
}

use alloy_primitives::{Address, Bytes, U256};

pub fn transact_fork(
  transact_to: Address,
  calldata: Option<Bytes>,
  value: Option<U256>,
  caller: Option<Address>,
  db: &mut CacheDB<AlloyDB<Http<Client>, Ethereum, Arc<alloy::providers::RootProvider<Http<Client>>>>>,
) -> Result<ExecutionResult, eyre::Error> {
  let mut tx = TxEnv::default();
  tx.transact_to = TransactTo::Call(transact_to);
  if let Some(calldata) = calldata {
      tx.data = calldata;
  }

  if let Some(caller) = caller {
      tx.caller = caller;
  }

  if let Some(value) = value {
      tx.value = value;
  }
  let mut evm = Evm::builder().with_db(db).with_tx_env(tx).build();

  let tx_res = evm.transact_commit()?;

  Ok(tx_res)
}

pub fn deploy(bytecode: Bytes, db: &mut CacheDB<AlloyDB<Http<Client>, Ethereum, Arc<alloy::providers::RootProvider<Http<Client>>>>>) -> Result<Address, eyre::Error> {
  let mut evm = Evm::builder()
      .with_db(db)
      .modify_tx_env(|tx| {
          tx.transact_to = TransactTo::Create;
          tx.data = bytecode;
      })
      .build();
  let result = evm.transact_commit()?;

  if let ExecutionResult::Success { output, .. } = result {
      let address = output
          .address()
          .ok_or(eyre::eyre!("No address in execution result output"))?;
      return Ok(*address);
  } else {
      Err(eyre::eyre!("Execution failed {:?}", result))
  }
}

#[cfg(test)]
mod test {

    use crate::compile;

    use super::*;
    use alloy_primitives::address;
    use alloy_primitives::hex;
    use alloy_sol_types::sol;
    use alloy_sol_types::SolCall;
    use revm::primitives::ExecutionResult;

    #[test]
    fn test_execute() {
      let solidity_code = r#"
      pragma solidity ^0.8.0;
      interface INFT {
        function ownerOf(uint) external returns (address);
      }

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

  let result = compile::solidity::compile(solidity_code);
  let (_, bytecode) = result.unwrap();

  sol! {
    function set(uint256 x) public;
  }

  let encoded = setCall { x: U256::ZERO }.abi_encode();
  let b = Bytecode::new_raw(hex::decode(bytecode).unwrap().into());
  let e: alloy_primitives::Bytes = encoded.into();

        let result = execute_calldatas_fork(b, vec![Call{calldata: Some(e), value: None, caller: None}]);

        // Print the result and gas used
        if let ExecutionResult::Success {
          gas_used, output, ..
        } = result.unwrap().first().unwrap().to_owned()
        {
            println!("With storage - Gas used: {}", gas_used);
            println!("With storage - Output: {:?}", output);
            assert!(gas_used > 0, "Gas used should be greater than 0");
        } else {
            panic!("Execution failed.");
        }
    }
  }
