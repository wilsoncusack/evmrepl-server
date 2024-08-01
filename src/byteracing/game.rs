use alloy_primitives::Bytes;
use alloy_sol_types::{sol, SolCall, SolValue};
use eyre::eyre;
use revm::{db::CacheDB, InMemoryDB};
use revm_primitives::{Address, ExecutionResult};
use serde::Serialize;

use crate::gas::{deploy, transact};

#[derive(Clone, Debug)]
pub struct Game {
    map: Map,
    cur_position: Position,
    cur_context: Bytes,
    car_address: Address,
    gas_used: u64,
    path: Path,
    db: CacheDB<InMemoryDB>,
    outcome: Option<RaceOutcome>,
    message: Option<String>,
}

#[derive(Clone, Debug, Copy, PartialEq, Serialize)]
pub enum RaceOutcome {
    Finish,
    Crash,
    Revert,
    Halt,
    MaxGas,
}

#[derive(Clone, Debug, Copy, Serialize)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

type Path = Vec<Position>;

pub type Map = Vec<Vec<i8>>;

#[derive(Debug, Serialize)]
pub struct RaceResult {
    pub outcome: RaceOutcome,
    pub path: Path,
    pub gas_used: u64,
    pub message: Option<String>,
}

sol! {
  enum Move {
    Up,
    Down,
    Left,
    Right
  }

  struct MapPosition {
    uint64 x;
    uint64 y;
  }

  function getNextMove(int8[][] calldata map, bytes calldata prevContext) external returns (Move move, bytes memory nextContext);
}

impl Game {
    pub fn new(map: Map, car: Bytes, start_position: Position) -> Result<Self, eyre::Error> {
        let mut db = CacheDB::new(InMemoryDB::default());
        let car_address = deploy(car, &mut db)?;
        Ok(Self {
            map,
            cur_position: start_position,
            car_address,
            gas_used: 0,
            path: vec![start_position],
            cur_context: MapPosition {
                x: start_position.x as u64,
                y: start_position.y as u64,
            }
            .abi_encode()
            .into(),
            db,
            outcome: None,
            message: None,
        })
    }

    pub fn run(mut self) -> Result<RaceResult, eyre::Error> {
        while self.outcome.is_none() {
            println!("top of run loop");
            let result = self.do_move()?;
            self.handle_result(result)?;
        }

        Ok(RaceResult {
            outcome: self.outcome.expect("No outcome"),
            path: self.path,
            gas_used: self.gas_used,
            message: self.message,
        })
    }

    pub fn handle_result(&mut self, result: ExecutionResult) -> Result<(), eyre::Error> {
        match result {
            ExecutionResult::Halt { reason, gas_used } => {
                println!("result halt");
                self.gas_used += gas_used;
                self.message = Some(format!("{:?}", reason));
                self.outcome = Some(RaceOutcome::Halt);
            }
            ExecutionResult::Revert { gas_used, output } => {
                println!("result revert");
                self.gas_used += gas_used;
                self.message = Some(output.to_string());
                self.outcome = Some(RaceOutcome::Revert);
            }
            ExecutionResult::Success {
                gas_used,
                gas_refunded,
                output,
                ..
            } => {
                println!("result success");
                self.gas_used += gas_used - gas_refunded;
                if self.gas_used > 2_000_000 {
                    self.outcome = Some(RaceOutcome::MaxGas);
                    self.message = Some("Max gas 2M".to_string());
                }
                let call_result = getNextMoveCall::abi_decode_returns(output.data(), false)?;
                self.update_position(call_result.r#move)?;
                // game may be over based on position, out of bounds
                // and we don't want to add current position again
                if self.outcome.is_some() {
                    return Ok(());
                }
                self.path.push(self.cur_position);
                self.check_game_over()?;
                self.cur_context = call_result.nextContext;
            }
        }
        Ok(())
    }

    pub fn do_move(&mut self) -> Result<ExecutionResult, eyre::Error> {
        let calldata = getNextMoveCall {
            map: self.map.clone(),
            prevContext: self.cur_context.clone(),
        }
        .abi_encode();
        let result = transact(
            self.car_address,
            Some(calldata.into()),
            None,
            None,
            &mut self.db,
        )?;
        Ok(result)
    }

    pub fn update_position(&mut self, r#move: Move) -> Result<(), eyre::Error> {
        match r#move {
            Move::Up => {
                if self.cur_position.y == 0 {
                    self.outcome = Some(RaceOutcome::Crash)
                } else {
                    self.cur_position = Position {
                        x: self.cur_position.x,
                        y: self.cur_position.y - 1,
                    }
                }
            }
            Move::Down => {
                if self.cur_position.y == self.map.len() - 1 {
                    self.outcome = Some(RaceOutcome::Crash)
                } else {
                    self.cur_position = Position {
                        x: self.cur_position.x,
                        y: self.cur_position.y + 1,
                    }
                }
            }
            Move::Left => {
                if self.cur_position.x == 0 {
                    self.outcome = Some(RaceOutcome::Crash)
                } else {
                    self.cur_position = Position {
                        x: self.cur_position.x - 1,
                        y: self.cur_position.y,
                    }
                }
            }
            Move::Right => {
                if self.cur_position.x == self.map[0].len() - 1 {
                    self.outcome = Some(RaceOutcome::Crash)
                } else {
                    self.cur_position = Position {
                        x: self.cur_position.x + 1,
                        y: self.cur_position.y,
                    }
                }
            }
            Move::__Invalid => return Err(eyre!("Invalid move")),
        }
        Ok(())
    }

    pub fn check_game_over(&mut self) -> Result<(), eyre::Error> {
        if self.cur_position.y >= self.map.len() || self.cur_position.x >= self.map[0].len() {
            self.outcome = Some(RaceOutcome::Crash);
            return Ok(());
        } else {
            let val = self.map[self.cur_position.y][self.cur_position.x];

            match val {
                0 => {}                                         // continue reacing
                -1 => self.outcome = Some(RaceOutcome::Finish), // Finish line
                _ => self.outcome = Some(RaceOutcome::Crash),   // Obstacle or out of bounds
            }
            Ok(())
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::compile::solidity::compile;

//     use super::*;
//     use alloy_primitives::Bytes;

//     fn get_car_bytecode() -> Result<Bytes, eyre::Error> {
//         let solidity_code = r#"
//         pragma solidity 0.8.26;

//         contract Car {
//             enum Move { Up, Down, Left, Right }

//             function getNextMove(int8[][] calldata map, bytes calldata prevContext) external returns (Move move, bytes memory nextContext)
//             {
//                 // Simple logic: always move right
//                 return (Move.Right, "");
//             }
//         }
//         "#;

//         let compile_result = compile(solidity_code)?;
//         let contract = compile_result.contracts.find_first("Car").unwrap();
//         let bytecode = contract.bytecode().unwrap();
//         Ok(bytecode.clone())
//     }

//     #[test]
//     fn test_game_creation() -> Result<(), eyre::Error> {
//         let map = vec![vec![0, 0, 0], vec![0, 0, 0], vec![0, 0, -1]];
//         let car_bytecode = get_car_bytecode()?;
//         let start_position = Position { x: 0, y: 0 };

//         let game = Game::new(map, car_bytecode, start_position)?;

//         assert_eq!(game.cur_position.x, 0);
//         assert_eq!(game.cur_position.y, 0);
//         assert_eq!(game.gas_used, 0);
//         assert_eq!(game.path.len(), 1);
//         assert!(game.outcome.is_none());

//         Ok(())
//     }

//     #[test]
//     fn test_game_run() -> Result<(), eyre::Error> {
//         let map = vec![vec![0, 0, 0], vec![0, 0, 0], vec![0, 0, -1]];
//         let car_bytecode = get_car_bytecode()?;
//         let start_position = Position { x: 0, y: 0 };

//         let game = Game::new(map, car_bytecode, start_position)?;
//         let result = game.run()?;

//         assert_eq!(result.outcome, RaceOutcome::Crash);
//         assert_eq!(result.path.len(), 3);
//         assert!(result.gas_used > 0);

//         Ok(())
//     }

//     #[test]
//     fn test_game_crash() -> Result<(), eyre::Error> {
//         let map = vec![vec![0, 0, 1], vec![0, 0, 0], vec![0, 0, -1]];
//         let car_bytecode = get_car_bytecode()?;
//         let start_position = Position { x: 0, y: 0 };

//         let game = Game::new(map, car_bytecode, start_position)?;
//         let result = game.run()?;

//         assert_eq!(result.outcome, RaceOutcome::Crash);
//         assert_eq!(result.path.len(), 3);
//         assert!(result.gas_used > 0);

//         Ok(())
//     }
// }
