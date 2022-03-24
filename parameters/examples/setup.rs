// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use snarkvm_algorithms::{
    crypto_hash::sha256::sha256,
    snark::marlin::{ahp::AHPForR1CS, MarlinHidingMode},
    CRH,
    SNARK,
    SRS,
};
use snarkvm_dpc::{InnerCircuit, InputCircuit, Network, OutputCircuit, PoSWScheme, ValueCheckCircuit};
use snarkvm_utilities::{FromBytes, ToBytes, ToMinimalBits};

use anyhow::Result;
use rand::{prelude::ThreadRng, thread_rng};
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

fn checksum(bytes: &[u8]) -> String {
    hex::encode(sha256(bytes))
}

fn versioned_filename(filename: &str, checksum: &str) -> String {
    match checksum.get(0..7) {
        Some(sum) => format!("{}.{}", filename, sum),
        _ => filename.to_string(),
    }
}

/// Writes the given bytes to the given versioned filename.
fn write_remote(filename: &str, version: &str, bytes: &[u8]) -> Result<()> {
    let mut file = BufWriter::new(File::create(PathBuf::from(&versioned_filename(filename, version)))?);
    file.write_all(bytes)?;
    Ok(())
}

/// Writes the given bytes to the given filename.
fn write_local(filename: &str, bytes: &[u8]) -> Result<()> {
    let mut file = BufWriter::new(File::create(PathBuf::from(filename))?);
    file.write_all(bytes)?;
    Ok(())
}

/// Writes the given metadata as JSON to the given filename.
fn write_metadata(filename: &str, metadata: &Value) -> Result<()> {
    let mut file = BufWriter::new(File::create(PathBuf::from(filename))?);
    file.write_all(&serde_json::to_vec_pretty(metadata)?)?;
    Ok(())
}

/// Runs a universal SRS setup.
pub fn universal_setup<N: Network>() -> Result<()> {
    const UNIVERSAL_METADATA: &str = "universal.metadata";
    const UNIVERSAL_SRS: &str = "universal.srs";

    let max_degree =
        AHPForR1CS::<<N as Network>::InnerScalarField, MarlinHidingMode>::max_degree(2000000, 4000000, 8000000)
            .unwrap();
    let universal_srs = <<N as Network>::ProgramSNARK as SNARK>::universal_setup(&max_degree, &mut thread_rng())?;
    let universal_srs = universal_srs.to_bytes_le()?;

    let universal_checksum = checksum(&universal_srs);
    let universal_metadata = json!({
        "srs_checksum": universal_checksum,
        "srs_size": universal_srs.len()
    });

    println!("{}", serde_json::to_string_pretty(&universal_metadata)?);
    write_metadata(UNIVERSAL_METADATA, &universal_metadata)?;
    write_remote(UNIVERSAL_SRS, &universal_checksum, &universal_srs)?;

    Ok(())
}

/// Runs the inner circuit setup.
pub fn inner_setup<N: Network>() -> Result<()> {
    const INNER_CIRCUIT_METADATA: &str = "inner.metadata";
    const INNER_PROVING_KEY: &str = "inner.proving";
    const INNER_VERIFYING_KEY: &str = "inner.verifying";

    let (inner_proving_key, inner_verifying_key) =
        N::InnerSNARK::setup(&InnerCircuit::<N>::blank(), &mut SRS::CircuitSpecific(&mut thread_rng()))?;

    let inner_circuit_id =
        hex::encode(N::inner_circuit_id_crh().hash(&inner_verifying_key.to_minimal_bits())?.to_bytes_le()?);
    let inner_proving_key = inner_proving_key.to_bytes_le()?;
    let inner_proving_checksum = checksum(&inner_proving_key);
    let inner_verifying_key = inner_verifying_key.to_bytes_le()?;

    let inner_metadata = json!({
        "proving_checksum": inner_proving_checksum,
        "proving_size": inner_proving_key.len(),
        "verifying_checksum": checksum(&inner_verifying_key),
        "verifying_size": inner_verifying_key.len(),
        "circuit_id": inner_circuit_id
    });

    println!("{}", serde_json::to_string_pretty(&inner_metadata)?);
    write_metadata(INNER_CIRCUIT_METADATA, &inner_metadata)?;
    write_remote(INNER_PROVING_KEY, &inner_proving_checksum, &inner_proving_key)?;
    write_local(INNER_VERIFYING_KEY, &inner_verifying_key)?;

    Ok(())
}

/// Runs the input circuit setup.
pub fn input_setup<N: Network>() -> Result<()> {
    const INPUT_CIRCUIT_METADATA: &str = "input.metadata";
    const INPUT_PROVING_KEY: &str = "input.proving";
    const INPUT_VERIFYING_KEY: &str = "input.verifying";

    let (input_proving_key, input_verifying_key) =
        N::InputSNARK::setup(&InputCircuit::<N>::blank(), &mut SRS::CircuitSpecific(&mut thread_rng()))?;

    let input_circuit_id =
        hex::encode(N::input_circuit_id_crh().hash(&input_verifying_key.to_minimal_bits())?.to_bytes_le()?);
    let input_proving_key = input_proving_key.to_bytes_le()?;
    let input_proving_checksum = checksum(&input_proving_key);
    let input_verifying_key = input_verifying_key.to_bytes_le()?;

    let input_metadata = json!({
        "proving_checksum": input_proving_checksum,
        "proving_size": input_proving_key.len(),
        "verifying_checksum": checksum(&input_verifying_key),
        "verifying_size": input_verifying_key.len(),
        "circuit_id": input_circuit_id
    });

    println!("{}", serde_json::to_string_pretty(&input_metadata)?);
    write_metadata(INPUT_CIRCUIT_METADATA, &input_metadata)?;
    write_remote(INPUT_PROVING_KEY, &input_proving_checksum, &input_proving_key)?;
    write_local(INPUT_VERIFYING_KEY, &input_verifying_key)?;

    Ok(())
}

/// Runs the output circuit setup.
pub fn output_setup<N: Network>() -> Result<()> {
    const OUTPUT_CIRCUIT_METADATA: &str = "output.metadata";
    const OUTPUT_PROVING_KEY: &str = "output.proving";
    const OUTPUT_VERIFYING_KEY: &str = "output.verifying";

    let (output_proving_key, output_verifying_key) =
        N::OutputSNARK::setup(&OutputCircuit::<N>::blank(), &mut SRS::CircuitSpecific(&mut thread_rng()))?;

    let output_circuit_id =
        hex::encode(N::output_circuit_id_crh().hash(&output_verifying_key.to_minimal_bits())?.to_bytes_le()?);
    let output_proving_key = output_proving_key.to_bytes_le()?;
    let output_proving_checksum = checksum(&output_proving_key);
    let output_verifying_key = output_verifying_key.to_bytes_le()?;

    let output_metadata = json!({
        "proving_checksum": output_proving_checksum,
        "proving_size": output_proving_key.len(),
        "verifying_checksum": checksum(&output_verifying_key),
        "verifying_size": output_verifying_key.len(),
        "circuit_id": output_circuit_id
    });

    println!("{}", serde_json::to_string_pretty(&output_metadata)?);
    write_metadata(OUTPUT_CIRCUIT_METADATA, &output_metadata)?;
    write_remote(OUTPUT_PROVING_KEY, &output_proving_checksum, &output_proving_key)?;
    write_local(OUTPUT_VERIFYING_KEY, &output_verifying_key)?;

    Ok(())
}

/// Runs the value check circuit setup.
pub fn value_check_setup<N: Network>() -> Result<()> {
    const VALUE_CHECK_CIRCUIT_METADATA: &str = "value_check.metadata";
    const VALUE_CHECK_PROVING_KEY: &str = "value_check.proving";
    const VALUE_CHECK_VERIFYING_KEY: &str = "value_check.verifying";

    let (value_check_proving_key, value_check_verifying_key) =
        N::ValueCheckSNARK::setup(&ValueCheckCircuit::<N>::blank(), &mut SRS::CircuitSpecific(&mut thread_rng()))?;

    let value_check_circuit_id =
        hex::encode(N::value_check_circuit_id_crh().hash(&value_check_verifying_key.to_minimal_bits())?.to_bytes_le()?);
    let value_check_proving_key = value_check_proving_key.to_bytes_le()?;
    let value_check_proving_checksum = checksum(&value_check_proving_key);
    let value_check_verifying_key = value_check_verifying_key.to_bytes_le()?;

    let value_check_metadata = json!({
        "proving_checksum": value_check_proving_checksum,
        "proving_size": value_check_proving_key.len(),
        "verifying_checksum": checksum(&value_check_verifying_key),
        "verifying_size": value_check_verifying_key.len(),
        "circuit_id": value_check_circuit_id
    });

    println!("{}", serde_json::to_string_pretty(&value_check_metadata)?);
    write_metadata(VALUE_CHECK_CIRCUIT_METADATA, &value_check_metadata)?;
    write_remote(VALUE_CHECK_PROVING_KEY, &value_check_proving_checksum, &value_check_proving_key)?;
    write_local(VALUE_CHECK_VERIFYING_KEY, &value_check_verifying_key)?;

    Ok(())
}

/// Runs the PoSW circuit setup.
pub fn posw_setup<N: Network>() -> Result<()> {
    const POSW_CIRCUIT_METADATA: &str = "posw.metadata";
    const POSW_PROVING_KEY: &str = "posw.proving";
    const POSW_VERIFYING_KEY: &str = "posw.verifying";

    // TODO: decide the size of the universal setup
    let max_degree =
        AHPForR1CS::<<N as Network>::InnerScalarField, MarlinHidingMode>::max_degree(40000, 40000, 60000).unwrap();
    let universal_srs = <<N as Network>::PoSWSNARK as SNARK>::universal_setup(&max_degree, &mut thread_rng())?;
    let srs_bytes = universal_srs.to_bytes_le()?;
    println!("srs\n\tsize - {}", srs_bytes.len());

    let posw = <N::PoSW as PoSWScheme<N>>::setup::<ThreadRng>(&mut SRS::<ThreadRng, _>::Universal(
        &FromBytes::read_le(&srs_bytes[..])?,
    ))?;

    let posw_proving_key = posw.proving_key().as_ref().expect("posw_proving_key is missing").to_bytes_le()?;
    let posw_proving_checksum = checksum(&posw_proving_key);
    let posw_verifying_key = posw.verifying_key().to_bytes_le()?;

    let posw_metadata = json!({
        "proving_checksum": posw_proving_checksum,
        "proving_size": posw_proving_key.len(),
        "verifying_checksum": checksum(&posw_verifying_key),
        "verifying_size": posw_verifying_key.len(),
    });

    println!("{}", serde_json::to_string_pretty(&posw_metadata)?);
    write_metadata(POSW_CIRCUIT_METADATA, &posw_metadata)?;
    write_remote(POSW_PROVING_KEY, &posw_proving_checksum, &posw_proving_key)?;
    write_local(POSW_VERIFYING_KEY, &posw_verifying_key)?;

    Ok(())
}

/// Run the following command to perform a setup.
/// `cargo run --example setup [parameter] [network]`
pub fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Invalid number of arguments. Given: {} - Required: 2", args.len() - 1);
        return Ok(());
    }

    match args[1].as_str() {
        "inner" => match args[2].as_str() {
            "testnet1" => inner_setup::<snarkvm_dpc::testnet1::Testnet1>()?,
            "testnet2" => inner_setup::<snarkvm_dpc::testnet2::Testnet2>()?,
            _ => panic!("Invalid network"),
        },
        "posw" => match args[2].as_str() {
            "testnet1" => posw_setup::<snarkvm_dpc::testnet1::Testnet1>()?,
            "testnet2" => posw_setup::<snarkvm_dpc::testnet2::Testnet2>()?,
            _ => panic!("Invalid network"),
        },
        "universal" => match args[2].as_str() {
            "testnet1" => panic!("Testnet1 does not support a universal SRS"),
            "testnet2" => universal_setup::<snarkvm_dpc::testnet2::Testnet2>()?,
            _ => panic!("Invalid network"),
        },
        "input" => match args[2].as_str() {
            "testnet1" => input_setup::<snarkvm_dpc::testnet1::Testnet1>()?,
            "testnet2" => input_setup::<snarkvm_dpc::testnet2::Testnet2>()?,
            _ => panic!("Invalid network"),
        },
        "output" => match args[2].as_str() {
            "testnet1" => output_setup::<snarkvm_dpc::testnet1::Testnet1>()?,
            "testnet2" => output_setup::<snarkvm_dpc::testnet2::Testnet2>()?,
            _ => panic!("Invalid network"),
        },
        "value_check" => match args[2].as_str() {
            "testnet1" => value_check_setup::<snarkvm_dpc::testnet1::Testnet1>()?,
            "testnet2" => value_check_setup::<snarkvm_dpc::testnet2::Testnet2>()?,
            _ => panic!("Invalid network"),
        },
        _ => panic!("Invalid parameter"),
    };

    Ok(())
}
