use gas_exp::routes::{
    byterace_route, compile_solidity_route, execute_calldatas_fork_route, execute_calldatas_route,
};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    // Configure CORS options
    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_headers(AllowedHeaders::all())
        .allow_credentials(true);

    rocket::build().attach(cors.to_cors().unwrap()).mount(
        "/",
        routes![
            execute_calldatas_route,
            compile_solidity_route,
            execute_calldatas_fork_route,
            byterace_route
        ],
    )
}
