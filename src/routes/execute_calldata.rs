use crate::gas::execute_calldata;
use alloy_primitives::{
    hex::{self, FromHex},
    Address, U256,
};
use revm::primitives::{Bytecode, Bytes, ResultAndState};
use rocket::{post, response::status, serde::json::Json};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct ExecuteCalldataRequest {
    pub bytecode: String,
    pub calldata: Option<String>,
    pub value: Option<String>,
    pub caller: Option<String>,
}

#[post("/execute_calldata", format = "json", data = "<req>")]
pub fn execute_calldata_route(
    req: Json<ExecuteCalldataRequest>,
) -> Result<Json<ResultAndState>, status::BadRequest<Option<String>>> {
    let result = handle(req).map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}

fn handle(req: Json<ExecuteCalldataRequest>) -> Result<ResultAndState, eyre::Error> {
    let bytecode = hex::decode(&req.bytecode).map_err(|err| eyre::eyre!(err.to_string()))?;
    let calldata = decode(&req.calldata)?;
    let value = decode(&req.value)?;
    let caller = decode(&req.caller)?;

    let result = execute_calldata(Bytecode::new_raw(bytecode.into()), calldata, value, caller)
        .map_err(|err| eyre::eyre!(err.to_string()))?;
    Ok(result)
}

fn decode<T: FromStr>(value: &Option<String>) -> Result<Option<T>, eyre::Report>
where
    T::Err: std::fmt::Display,
{
    value
        .as_deref()
        .map(T::from_str)
        .transpose()
        .map_err(|err| eyre::eyre!(err.to_string()))
}
