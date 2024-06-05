use crate::gas::execute_calldata::execute_bytecode;
use alloy_primitives::{hex::{self, FromHex}, Address, U256};
use revm::primitives::{Bytecode, Bytes};
use rocket::{post, response::status, serde::json::Json};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Deserialize)]
pub struct ExecuteCalldataRequest {
    pub bytecode: String,
    pub calldata: Option<String>,
    pub value: Option<String>,
    pub caller: Option<String>,
}
#[derive(Serialize)]
pub struct ExecuteCallDataResponse {
  gas_used: u64
}

#[post("/execute_calldata", format = "json", data = "<req>")]
pub fn execute_calldata(req: Json<ExecuteCalldataRequest>) -> Result<Json<ExecuteCallDataResponse>, status::BadRequest<Option<String>>> {
    let result = handle(req).map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}

fn handle(req: Json<ExecuteCalldataRequest>) -> Result<ExecuteCallDataResponse, eyre::Error> {
    let bytecode = hex::decode(&req.bytecode).map_err(|err| eyre::eyre!(err.to_string()))?;
    let calldata = req.calldata.as_deref().map(Bytes::from_hex).transpose().map_err(|err| eyre::eyre!(err.to_string()))?;
    let value = req.value.as_deref().map(U256::from_str).transpose().map_err(|err| eyre::eyre!(err.to_string()))?;
    let caller = req.caller.as_deref().map(Address::from_str).transpose().map_err(|err| eyre::eyre!(err.to_string()))?;

    let gas_used = execute_bytecode(Bytecode::new_raw(bytecode.into()), calldata, value, caller)
        .map_err(|err| eyre::eyre!(err.to_string()))?;
    Ok(ExecuteCallDataResponse{gas_used})
}
