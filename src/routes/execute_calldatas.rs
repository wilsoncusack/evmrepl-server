use crate::gas::{execute_calldatas, Call};
use alloy_primitives::hex;
use revm::primitives::{Bytecode, ExecutionResult};
use rocket::{post, response::status, serde::json::Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ExecuteCalldatasRequest {
    pub bytecode: String,
    pub calls: Vec<Call>,
}

#[post("/execute_calldatas", format = "json", data = "<req>")]
pub fn execute_calldatas_route(
    req: Json<ExecuteCalldatasRequest>,
) -> Result<Json<Vec<ExecutionResult>>, status::BadRequest<Option<String>>> {
    let result = handle(req).map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}

fn handle(req: Json<ExecuteCalldatasRequest>) -> Result<Vec<ExecutionResult>, eyre::Error> {
    let bytecode = hex::decode(&req.bytecode).map_err(|err| eyre::eyre!(err.to_string()))?;
    let result = execute_calldatas(Bytecode::new_raw(bytecode.into()), req.calls.clone())
        .map_err(|err| eyre::eyre!(err.to_string()))?;
    Ok(result)
}
