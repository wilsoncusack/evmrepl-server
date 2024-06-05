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
