use crate::gas::{execute_calldatas_fork, ExecutionResult, ForkCall};
use alloy_primitives::Address;
use alloy_primitives::Bytes;
use rocket::{post, response::status, serde::json::Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ExecuteCalldatasRequest {
    pub bytecode: Bytes,
    pub address: Address,
    pub calls: Vec<ForkCall>,
}

#[post("/execute_calldatas_fork", format = "json", data = "<req>")]
pub async fn execute_calldatas_fork_route(
    req: Json<ExecuteCalldatasRequest>,
) -> Result<Json<Vec<ExecutionResult>>, status::BadRequest<Option<String>>> {
    let result = execute_calldatas_fork(req.bytecode.clone(), req.address, req.calls.clone())
        .await
        .map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}
