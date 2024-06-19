use alloy_primitives::{Address, Bytes};
use revm::{
    db::CacheDB,
    primitives::{ExecutionResult, TransactTo},
    Evm, InMemoryDB,
};

pub fn deploy(bytecode: Bytes, db: &mut CacheDB<InMemoryDB>) -> Result<Address, eyre::Error> {
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
