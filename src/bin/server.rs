use gas_exp::routes::{compile_solidity_route, execute_calldata_route};

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![execute_calldata_route, compile_solidity_route])
}
