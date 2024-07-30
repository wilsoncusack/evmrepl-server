use foundry_compilers::{
    contracts::VersionedContracts, multi::MultiCompilerError, Project, ProjectPathsConfig,
};
use serde::Serialize;
use std::fs;
use tempfile::{self, TempDir};

#[derive(Debug, Serialize)]
pub struct CompileResult {
    errors: Vec<MultiCompilerError>,
    contracts: VersionedContracts,
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

    println!("Solidity file written to: {:?}", file_path);

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
    // let errors = &output.output().contracts;
    Ok(CompileResult {
        errors: output.output().errors.clone(),
        contracts: output.output().contracts.clone(),
    })
    //     println!("Errors: {:?}", errors);

    // // Print the errors
    // for error in errors {
    //     println!("Error: {:?}", error);
    // }
    //     let results: Vec<Result<ContractData, eyre::Error>> = output
    //         .into_artifacts()
    //         .map(|a| {
    //             let abi = serde_json::to_string(&a.1.abi)
    //                 .map_err(|err| eyre::eyre!("No abi for artifact {:?}: {:?}", a.0.name, err))?;
    //             let bytecode = serde_json::to_string(&a.1.bytecode)
    //                 .map_err(|err| eyre::eyre!("No bytecode for artifact {:?}: {:?}", a.0.name, err))?;
    //             Ok(ContractData {
    //                 name: a.0.name,
    //                 abi: abi,
    //                 bytecode: bytecode,
    //             })
    //         }).collect();

    //     results.into_iter().collect()
}

fn parse_solc_errors(stderr: &str) -> Vec<SolcError> {
    let error_regex = Regex::new(r"(?m)^(Warning|Error): (.+)$").unwrap();
    let details_regex = Regex::new(r"(?ms)--> (.+?):(\d+):(\d+):\n((?:.|\n)*?)\n\n").unwrap();

    let mut errors = Vec::new();

    for error_cap in error_regex.captures_iter(stderr) {
        let error_type = match &error_cap[1] {
            "Warning" => ErrorType::Warning,
            _ => ErrorType::Error,
        };
        let message = error_cap[2].trim().to_string();

        let mut details = ErrorDetails {
            line: None,
            column: None,
            code_snippet: None,
        };

        // Look for details after the error message
        if let Some(details_cap) =
            details_regex.captures(&stderr[error_cap.get(0).unwrap().end()..])
        {
            details.line = details_cap[2].parse().ok();
            details.column = details_cap[3].parse().ok();
            details.code_snippet = Some(details_cap[4].trim().to_string());
        }

        errors.push(SolcError {
            error_type,
            message,
            details,
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spdx_warning() {
        let input = "Warning: SPDX license identifier not provided in source file. Before publishing, consider adding a comment containing \"SPDX-License-Identifier: <SPDX-License>\" to each source file. Use \"SPDX-License-Identifier: UNLICENSED\" for non-open-source code. Please see https://spdx.org for more information.\n--> /path/to/file.sol\n\n";
        let result = parse_solc_errors(input);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].error_type, ErrorType::Warning));
        assert!(result[0]
            .message
            .contains("SPDX license identifier not provided"));
        assert_eq!(result[0].details.line, None);
        assert_eq!(result[0].details.column, None);
    }

    #[test]
    fn test_parse_error_with_line_and_column() {
        let input = "Error: Expected ';' but got 'event'\n --> /path/to/file.sol:5:5:\n  |\n5 |     event StoredDataUpdated(uint);\n  |     ^^^^^\n\n";
        let result = parse_solc_errors(input);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].error_type, ErrorType::Error));
        assert_eq!(result[0].message, "Expected ';' but got 'event'");
        assert_eq!(result[0].details.line, Some(5));
        assert_eq!(result[0].details.column, Some(5));
        assert!(result[0]
            .details
            .code_snippet
            .clone()
            .unwrap()
            .contains("event StoredDataUpdated(uint);"));
    }

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
