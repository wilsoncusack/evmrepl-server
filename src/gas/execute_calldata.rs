use alloy_primitives::{address, keccak256, Address, U256};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{AccountInfo, Bytecode, Bytes, ResultAndState, TransactTo, TxEnv},
    Evm,
};

pub fn execute_calldata(
    bytecode: Bytecode,
    calldata: Option<Bytes>,
    value: Option<U256>,
    caller: Option<Address>,
) -> Result<ResultAndState, eyre::Error> {
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

    let tx_res = evm.transact()?;

    Ok(tx_res)
}

#[cfg(test)]
mod test {
    use super::*;
    use revm::primitives::ExecutionResult;

    #[test]
    fn test_execute_calldata_with_storage_operations() {
        // Example bytecode with storage operations (SSTORE and SLOAD)
        let bytecode = Bytecode::new_raw(
            vec![
                0x60, 0x00, // PUSH1 0x00
                0x60, 0x01, // PUSH1 0x01
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
        } = result.unwrap().result
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
        } = result.unwrap().result
        {
            println!("Without storage - Gas used: {}", gas_used);
            println!("Without storage - Output: {:?}", output);
            assert!(gas_used > 0, "Gas used should be greater than 0");
        } else {
            panic!("Execution failed.");
        }
    }
}
