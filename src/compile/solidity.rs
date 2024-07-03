use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize)]
pub struct SolcCompileResponse {
    pub data: Vec<ContractData>,
    pub errors: Vec<SolcError>,
}
#[derive(Debug, Clone, Serialize)]
pub enum ErrorType {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolcError {
    pub error_type: ErrorType,
    pub message: String,
    pub details: ErrorDetails,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub code_snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractData {
    pub name: String,
    pub abi: String,
    pub bytecode: String,
}

pub fn compile(code: &str) -> Result<SolcCompileResponse, eyre::Error> {
    // Create a temporary file to hold the Solidity code
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(code.as_bytes())?;

    // Compile the Solidity code using solc
    let output = Command::new("solc")
        .arg("--combined-json")
        .arg("bin,abi")
        .arg(temp_file.path())
        .output()?;

    if !output.status.success() {
        return Err(eyre::eyre!(format!(
            "solc failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    println!("output {:?}", output);

    Ok(SolcCompileResponse {
        data: parse_solc_out(output.stdout)?,
        errors: parse_solc_errors(&String::from_utf8_lossy(&output.stderr).to_string()),
    })
}

fn parse_solc_out(stdout: Vec<u8>) -> Result<Vec<ContractData>, eyre::Error> {
    if stdout.is_empty() {
        return Ok(Vec::default());
    }
    let solc_output: Value = serde_json::from_slice(&stdout)?;

    let contracts = solc_output
        .get("contracts")
        .ok_or(eyre::eyre!("No contracts key in solc output"))?
        .as_object()
        .ok_or(eyre::eyre!("Contracts is not an object"))?;

    let mut results = Vec::new();

    for (full_name, contract_data) in contracts {
        // Extract the contract name from the full name
        let name = full_name
            .split(':')
            .last()
            .ok_or(eyre::eyre!("Invalid contract name format"))?
            .to_string();

        let abi = contract_data
            .get("abi")
            .ok_or(eyre::eyre!("No abi in contract"))?
            .to_string();

        let bytecode = contract_data
            .get("bin")
            .ok_or(eyre::eyre!("No bin in contract"))?
            .as_str()
            .ok_or(eyre::eyre!("Bin is not a string"))?
            .to_string();

        results.push(ContractData {
            name,
            abi,
            bytecode,
        });
    }

    Ok(results)
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
        pragma solidity ^0.8.0;

        contract SimpleStorage {
            uint256 public storedData;

            function set(uint256 x) public {
                storedData = x;
            }

            function get() public view returns (uint256) {
                return storedData;
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
        assert!(result.is_ok());

        let contracts = result.unwrap().data;
        assert_eq!(contracts.len(), 2);

        let simple_storage = &contracts[1];
        assert!(simple_storage.name.contains("SimpleStorage"));
        assert!(simple_storage.abi.contains("storedData"));
        assert!(simple_storage.bytecode.starts_with("60"));

        let another_contract = &contracts[0];
        assert!(another_contract.name.contains("AnotherContract"));
        assert!(another_contract.abi.contains("message"));
        assert!(another_contract.bytecode.starts_with("60"));
    }

    #[test]
    fn test_compile_invalid_contract() {
        let invalid_solidity_code = r#"
        pragma solidity ^0.8.0;
        contract InvalidContract {
            uint256 public storedData
            function set(uint256 x) public {
                storedData = x;
            }
            function get() public view returns (uint256) {
                return storedData;
            }
        }
        "#;

        let result = compile(invalid_solidity_code);
        assert!(result.is_err());
    }
}
