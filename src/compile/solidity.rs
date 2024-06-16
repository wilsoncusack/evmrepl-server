use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

pub fn compile(code: &str) -> Result<(String, String), eyre::Error> {
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

    let solc_output: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let contracts = solc_output
        .get("contracts")
        .ok_or(eyre::eyre!("No contracts key in solc output"))?;
    let contract = contracts
        .as_object()
        .ok_or(eyre::eyre!("Contracts is not an object"))?
        .values()
        .next()
        .ok_or(eyre::eyre!("No contracts found"))?;

    println!("{:?}", contract);

    let abi = contract
        .get("abi")
        .ok_or(eyre::eyre!("No abi in contract"))?
        .to_string();
    let bytecode = contract
        .get("bin")
        .ok_or(eyre::eyre!("No bin in contract"))?
        .as_str()
        .ok_or(eyre::eyre!("Bin is not a string"))?
        .to_string();

    Ok((abi, bytecode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_valid_contract() {
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
        "#;

        let result = compile(solidity_code);
        assert!(result.is_ok());

        let (abi, bytecode) = result.unwrap();
        assert!(abi.contains("storedData"));
        assert!(bytecode.starts_with("6080"));
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
