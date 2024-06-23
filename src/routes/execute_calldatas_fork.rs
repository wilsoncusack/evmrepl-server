use crate::gas::{execute_calldatas_fork, ExecutionResult, ForkCall};
use alloy_primitives::Bytes;
use rocket::{post, response::status, serde::json::Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ExecuteCalldatasRequest {
    pub bytecode: Bytes,
    pub calls: Vec<ForkCall>,
}

#[post("/execute_calldatas_fork", format = "json", data = "<req>")]
pub async fn execute_calldatas_fork_route(
    req: Json<ExecuteCalldatasRequest>,
) -> Result<Json<Vec<ExecutionResult>>, status::BadRequest<Option<String>>> {
    let result = handle(req)
        .await
        .map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}

async fn handle(req: Json<ExecuteCalldatasRequest>) -> Result<Vec<ExecutionResult>, eyre::Error> {
    // let bytecode = hex::decode(&req.bytecode).map_err(|err| eyre::eyre!(err.to_string()))?;
    let result = execute_calldatas_fork(req.bytecode.clone(), req.calls.clone())
        .await
        .map_err(|err| eyre::eyre!(err.to_string()))?;
    Ok(result)
}
