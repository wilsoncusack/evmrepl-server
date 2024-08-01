use foundry_compilers::{
    contracts::VersionedContracts, multi::MultiCompilerError, Project, ProjectPathsConfig,
};
use serde::{Deserialize, Serialize};
use std::fs;
use tempfile::{self, TempDir};

#[derive(Deserialize)]
pub struct SolidityFile {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct CompileResult {
    pub errors: Vec<MultiCompilerError>,
    pub contracts: VersionedContracts,
}

pub fn compile(files: &[SolidityFile]) -> Result<CompileResult, eyre::Error> {
    // Create a temporary directory
    let temp_dir = TempDir::new()?;

    // Create a subdirectory for sources
    let sources_dir = temp_dir.path().join("src");
    fs::create_dir(&sources_dir)?;

    // Write each Solidity file to the sources directory
    for file in files {
        let file_path = sources_dir.join(&file.name);
        fs::write(&file_path, &file.content)?;
    }

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
        let files = vec![
            SolidityFile {
                name: "SimpleStorage.sol".to_string(),
                content: r#"
            pragma solidity 0.8.2;

            import "./AnotherContract.sol";

            contract SimpleStorage {
                uint256 public storedData;

                function set(uint256 x) public {
                    storedData = x;
                }

                function get() public view returns (uint256) {
                    return storedData;
                }
            }
            "#
                .to_string(),
            },
            SolidityFile {
                name: "AnotherContract.sol".to_string(),
                content: r#"
            pragma solidity ^0.8.1;

            contract AnotherContract {
                string public message;

                function setMessage(string memory _message) public {
                    message = _message;
                }
            }
            "#
                .to_string(),
            },
        ];

        let result = compile(&files);

        // assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        let compile_result = result.unwrap();

        // Check if there are no errors
        // assert!(compile_result.errors.is_empty(), "Compilation had errors: {:?}", compile_result.errors);

        // Check if both contracts are present in the output
        // assert!(compile_result.contracts.contains_key("SimpleStorage.sol"));
        // assert!(compile_result.contracts.contains_key("AnotherContract.sol"));

        // let simple_storage = &compile_result.contracts["SimpleStorage.sol"];
        // let another_contract = &compile_result.contracts["AnotherContract.sol"];

        // assert!(simple_storage.contains_key("SimpleStorage"));
        // assert!(another_contract.contains_key("AnotherContract"));

        println!("Compilation successful: {:?}", compile_result);
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
