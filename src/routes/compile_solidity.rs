use crate::compile::solidity::{compile, CompileResult, SolidityFile};
use rocket::{post, response::status, serde::json::Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CompileRequest {
    pub files: Vec<SolidityFile>,
}
#[post("/compile_solidity", format = "json", data = "<req>")]
pub fn compile_solidity_route(
    req: Json<CompileRequest>,
) -> Result<Json<CompileResult>, status::BadRequest<String>> {
    let result = compile(&req.files).map_err(|err| status::BadRequest(err.to_string()))?;

    Ok(Json(result))
}
