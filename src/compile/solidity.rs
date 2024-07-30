use foundry_compilers::{
    contracts::VersionedContracts, multi::MultiCompilerError, Project, ProjectPathsConfig,
};
use serde::Serialize;
use std::fs;
use tempfile::{self, TempDir};

#[derive(Debug, Serialize)]
pub struct CompileResult {
    pub errors: Vec<MultiCompilerError>,
    pub contracts: VersionedContracts,
}

pub fn compile(code: &str) -> Result<CompileResult, eyre::Error> {
    // Create a temporary directory
    let temp_dir = TempDir::new()?;

    // Create a subdirectory for sources
    let sources_dir = temp_dir.path().join("src");
    fs::create_dir(&sources_dir)?;

    // Write the Solidity code to a file in the sources directory
    let file_path = sources_dir.join("Contract.sol");
    fs::write(&file_path, code)?;

    let paths = ProjectPathsConfig::builder()
        .root(sources_dir.clone())
        .sources(sources_dir)
        .build()?;

    let project = Project::builder()
        .paths(paths)
        .ephemeral()
        .no_artifacts()
        .build(Default::default())?;

    let output = project.compile()?;

    Ok(CompileResult {
        errors: output.output().errors.clone(),
        contracts: output.output().contracts.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_valid_contracts() {
        let solidity_code = r#"
        pragma solidity 0.8.1;

        contract SimpleStorage {
            uint256 public storedData;

            function set(uint256 x) public {
                storedData = x;
            }

            function get() public view returns (uint256) {
                return storedData
            }
        }

        contract AnotherContract {
            string public message;

            function setMessage(string memory _message) public {
                message = _message;
            }
        }
        "#;

        let result = compile(solidity_code);
        println!("{:?}", result);
        // assert!(result.is_ok());

        // let contracts = result.unwrap();
        // assert_eq!(contracts.len(), 2);

        // let simple_storage = &contracts[1];
        // assert!(simple_storage.name.contains("SimpleStorage"));
        // assert!(simple_storage.abi.contains("storedData"));
        // assert!(simple_storage.bytecode.starts_with("60"));

        // let another_contract = &contracts[0];
        // assert!(another_contract.name.contains("AnotherContract"));
        // assert!(another_contract.abi.contains("message"));
        // assert!(another_contract.bytecode.starts_with("60"));
    }

    // #[test]
    // fn test_compile_invalid_contract() {
    //     let invalid_solidity_code = r#"
    //     pragma solidity ^0.8.0;
    //     contract InvalidContract {
    //         uint256 public storedData
    //         function set(uint256 x) public {
    //             storedData = x;
    //         }
    //         function get() public view returns (uint256) {
    //             return storedData;
    //         }
    //     }
    //     "#;

    //     let result = compile(invalid_solidity_code);
    //     assert!(result.is_err());
    //     println!("{:?}", result.err().unwrap());
    // }
}
