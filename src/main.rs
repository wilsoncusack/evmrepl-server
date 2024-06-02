use alloy_primitives::keccak256;
use revm::{db::{CacheDB, EmptyDB}, primitives::{address, hex, AccountInfo, Bytecode, TransactTo, U256}, Evm};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addy = address!("0000000000000000000000000000000000000001");
    let code = hex!("6080604052348015600f57600080fd5b5060043610603c5760003560e01c80633fb5c1cb1460415780638381f58a146053578063d09de08a14606d575b600080fd5b6051604c36600460ac565b600055565b005b605b60005481565b60405190815260200160405180910390f35b6051600080549080607c8360c4565b90915550506040517f648b7ce85d785f5dfdc6f193d1a20497833c776760c9a848442e7e44ee34632c90600090a1565b60006020828403121560bd57600080fd5b5035919050565b60006001820160e357634e487b7160e01b600052601160045260246000fd5b506001019056fea2646970667358221220fe7ae1d337cf7b1caf613bebfadd54be284af0e316b225386585026d4bd4fde764736f6c63430008170033");
    let code_hash = keccak256(code);
    // printline 
    println!("code_hash: {:?}", code_hash); 

    let mut account = AccountInfo::new(
        U256::ZERO,
        0,
        code_hash, 
        Bytecode::new_raw(code.into())
    );
    let mut cache_db = CacheDB::new(EmptyDB::default());
    cache_db.insert_contract(&mut account);
    cache_db.insert_account_info(addy, account);
    let mut evm = Evm::builder()
    .with_db(cache_db)
    .modify_tx_env(|tx| {
        // fill in missing bits of env struct
        // change that to whatever caller you want to be
        tx.caller = address!("0000000000000000000000000000000000000000");
        // account you want to transact with
        tx.transact_to = TransactTo::Call(addy);
        // calldata formed via abigen
        tx.data = hex!("d09de08a").into();
        // transaction value in wei
        tx.value = U256::from(0);
    })
    .build();
    let ref_tx = evm.transact().unwrap();
    // select ExecutionResult struct
    let result = ref_tx.result;
    println!("result: {:?}", result); 
    Ok(())
}
