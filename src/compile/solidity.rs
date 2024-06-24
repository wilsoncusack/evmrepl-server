use serde::Serialize;
use serde_json::Value;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize)]
pub struct ContractData {
    pub name: String,
    pub abi: String,
    pub bytecode: String,
}

pub fn compile(code: &str) -> Result<Vec<ContractData>, eyre::Error> {
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

    let solc_output: Value = serde_json::from_slice(&output.stdout)?;
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

#[cfg(test)]
mod tests {
    use super::*;

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

        let contracts = result.unwrap();
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
