use alloy_primitives::{Address, Bytes, U256};
use revm::{
    db::CacheDB,
    primitives::{Bytecode, ExecutionResult},
    InMemoryDB,
};
use serde::Deserialize;

use super::{deploy, transact};

#[derive(Deserialize, Clone)]
pub struct Call {
    pub calldata: Option<Bytes>,
    pub value: Option<U256>,
    pub caller: Option<Address>,
}

pub fn execute_calldatas(
    bytecode: Bytecode,
    calls: Vec<Call>,
) -> Result<Vec<ExecutionResult>, eyre::Error> {
    let mut db = CacheDB::new(InMemoryDB::default());

    let address = deploy(bytecode.bytes(), &mut db)?;

    calls
        .into_iter()
        .map(|call| transact(address, call.calldata, call.value, call.caller, &mut db))
        .collect()
}
