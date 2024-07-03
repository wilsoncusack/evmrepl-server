use crate::compile::solidity::{compile, SolcCompileResponse};
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
) -> Result<Json<SolcCompileResponse>, status::BadRequest<Option<String>>> {
    let result = compile(&req.code).map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}
