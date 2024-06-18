use alloy_primitives::{address, keccak256, Address, U256};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{AccountInfo, Bytecode, Bytes, ExecutionResult, TransactTo, TxEnv},
    Evm,
};

pub fn execute_calldata(
    bytecode: Bytecode,
    calldata: Option<Bytes>,
    value: Option<U256>,
    caller: Option<Address>,
) -> Result<ExecutionResult, eyre::Error> {
    let dummy_address = address!("1000000000000000000000000000000000000000");
    let code_hash = keccak256(&bytecode.bytes());

    let account = AccountInfo::new(U256::ZERO, 0, code_hash, bytecode.into());

    let mut db = CacheDB::new(EmptyDB::default());
    db.insert_account_info(dummy_address, account);

    let mut tx = TxEnv::default();
    tx.transact_to = TransactTo::Call(dummy_address);
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

#[cfg(test)]
mod test {

    use crate::compile;

    use super::*;
    use alloy_sol_types::sol;
    use alloy_sol_types::SolCall;
    use revm::primitives::ExecutionResult;

    #[test]
    fn test_execute_calldata_with_storage_operations() {
        // Example bytecode with storage operations (SSTORE and SLOAD)
        let bytecode = Bytecode::new_raw(
            vec![
                0x60, 0x01, // PUSH1 0x00
                0x60, 0x00, // PUSH1 0x01
                0x55, // SSTORE (store 1 at storage slot 0)
                0x60, 0x00, // PUSH1 0x00
                0x54, // SLOAD (load value at storage slot 0)
                0x60, 0x00, // PUSH1 0x00
                0xF3, // RETURN
            ]
            .into(),
        );

        let calldata = Some(Bytes::from(vec![]));
        let value = Some(U256::from(0));
        let caller = Some(address!("b000000000000000000000000000000000000000"));

        let result = execute_calldata(bytecode, calldata, value, caller);

        // Print the result and gas used
        if let ExecutionResult::Success {
            gas_used, output, ..
        } = result.unwrap()
        {
            println!("With storage - Gas used: {}", gas_used);
            println!("With storage - Output: {:?}", output);
            assert!(gas_used > 0, "Gas used should be greater than 0");
        } else {
            panic!("Execution failed.");
        }
    }

    #[test]
    fn test_execute_calldata_without_storage_operations() {
        // Example bytecode without storage operations
        let bytecode = Bytecode::new_raw(
            vec![
                0x60, 0x00, // PUSH1 0x00
                0x60, 0x01, // PUSH1 0x01
                0x60, 0x00, // PUSH1 0x00
                0xF3, // RETURN
            ]
            .into(),
        );

        let calldata = Some(Bytes::from(vec![]));
        let value = Some(U256::from(0));
        let caller = Some(address!("b000000000000000000000000000000000000000"));

        let result = execute_calldata(bytecode, calldata, value, caller);

        // Print the result and gas used
        if let ExecutionResult::Success {
            gas_used, output, ..
        } = result.unwrap()
        {
            println!("Without storage - Gas used: {}", gas_used);
            println!("Without storage - Output: {:?}", output);
            assert!(gas_used > 0, "Gas used should be greater than 0");
        } else {
            panic!("Execution failed.");
        }
    }

    sol! {
      function set(uint256 x) external;
    }

    #[test]
    fn test_execute_solidity() {
        let solidity_code = r#"
            pragma solidity ^0.8.0;
            contract SimpleStorage {
                uint256 public storedData;

                function set(uint256 x) public {
                    storedData = x;
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
        let b = Bytecode::new_raw(bytecode.into());
        let e: alloy_primitives::Bytes = encoded.into();
        println!("{:?}", execute_calldata(b, Some(e), None, None));
    }
}
