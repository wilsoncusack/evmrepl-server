use crate::byteracing::{Game, Map, Position, RaceResult};
use alloy_primitives::Bytes;
use rocket::{post, response::status, serde::json::Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ByteRaceRequest {
    map: Map,
    bytecode: Bytes,
}

#[post("/byterace", format = "json", data = "<req>")]
pub fn byterace_route(
    req: Json<ByteRaceRequest>,
) -> Result<Json<RaceResult>, status::BadRequest<Option<String>>> {
    let game = Game::new(
        req.map.clone(),
        req.bytecode.clone(),
        Position { x: 0, y: 0 },
    )
    .map_err(|err| status::BadRequest(Some(err.to_string())))?;
    let result = game
        .run()
        .map_err(|err| status::BadRequest(Some(err.to_string())))?;
    Ok(Json(result))
}
