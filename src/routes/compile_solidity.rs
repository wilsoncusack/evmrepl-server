use crate::compile::solidity::compile;
use eyre::Error;
use rocket::{post, response::status, serde::json::Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CompileRequest {
    pub code: String,
}

#[derive(serde::Serialize)]
pub struct CompileResponse {
    pub abi: String,
    pub bytecode: String,
}

#[post("/compile_solidity", format = "json", data = "<req>")]
pub fn compile_solidity_route(
    req: Json<CompileRequest>,
) -> Result<Json<CompileResponse>, status::BadRequest<Option<String>>> {
    let result = handle(req).map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}

fn handle(req: Json<CompileRequest>) -> Result<CompileResponse, Error> {
    let (abi, bytecode) = compile(&req.code)?;
    Ok(CompileResponse { abi, bytecode })
}
