use alloy_primitives::{Address, Bytes, U256};
use revm::{
    db::CacheDB,
    primitives::{ExecutionResult, TransactTo, TxEnv},
    Evm, InMemoryDB,
};

pub fn transact(
    transact_to: Address,
    calldata: Option<Bytes>,
    value: Option<U256>,
    caller: Option<Address>,
    db: &mut CacheDB<InMemoryDB>,
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
