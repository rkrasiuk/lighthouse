use clap::ArgMatches;
use clap_utils::parse_required;
use eth2_network_config::Eth2NetworkConfig;
use serde::Serialize;
use snap::raw::Decoder;
use ssz::Decode;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use types::*;

enum OutputFormat {
    Json,
    Yaml,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(Self::Json),
            "yaml" => Ok(Self::Yaml),
            _ => Err(format!("Invalid output format \"{}\"", s)),
        }
    }
}

pub fn run_parse_ssz<T: EthSpec>(
    network_config: Eth2NetworkConfig,
    matches: &ArgMatches,
) -> Result<(), String> {
    let type_str = matches.value_of("type").ok_or("No type supplied")?;
    let filename = matches.value_of("ssz-file").ok_or("No file supplied")?;
    let format = parse_required(matches, "format")?;

    let bytes = if filename.ends_with("ssz_snappy") {
        let bytes = fs::read(filename).unwrap();
        let mut decoder = Decoder::new();
        decoder.decompress_vec(&bytes).unwrap()
    } else {
        let mut bytes = vec![];
        let mut file =
            File::open(filename).map_err(|e| format!("Unable to open {}: {}", filename, e))?;
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("Unable to read {}: {}", filename, e))?;
        bytes
    };

    let spec = &network_config.chain_spec::<T>()?;
    info!(
        "Using {} network config ({} preset)",
        spec.config_name.as_deref().unwrap_or("unknown"),
        T::spec_name()
    );
    info!("Type: {type_str}");

    // More fork-specific decoders may need to be added in future, but shouldn't be 100% necessary,
    // as the fork-generic decoder will always be available (requires correct --network flag).
    match type_str {
        "SignedBeaconBlock" => decode_and_print::<SignedBeaconBlock<T>>(
            &bytes,
            |bytes| SignedBeaconBlock::from_ssz_bytes(bytes, spec),
            format,
        )?,
        "SignedBeaconBlockBase" | "SignedBeaconBlockPhase0" => {
            decode_and_print(&bytes, SignedBeaconBlockBase::<T>::from_ssz_bytes, format)?
        }
        "SignedBeaconBlockAltair" => {
            decode_and_print(&bytes, SignedBeaconBlockAltair::<T>::from_ssz_bytes, format)?
        }
        "SignedBeaconBlockMerge" | "SignedBeaconBlockBellatrix" => {
            decode_and_print(&bytes, SignedBeaconBlockMerge::<T>::from_ssz_bytes, format)?
        }
        "SignedBeaconBlockCapella" => decode_and_print(
            &bytes,
            SignedBeaconBlockCapella::<T>::from_ssz_bytes,
            format,
        )?,
        "BeaconState" => decode_and_print::<BeaconState<T>>(
            &bytes,
            |bytes| BeaconState::from_ssz_bytes(bytes, spec),
            format,
        )?,
        "BeaconStateBase" | "BeaconStatePhase0" => {
            decode_and_print(&bytes, BeaconStateBase::<T>::from_ssz_bytes, format)?
        }
        "BeaconStateAltair" => {
            decode_and_print(&bytes, BeaconStateAltair::<T>::from_ssz_bytes, format)?
        }
        "BeaconStateMerge" | "BeaconStateBellatrix" => {
            decode_and_print(&bytes, BeaconStateMerge::<T>::from_ssz_bytes, format)?
        }
        "BeaconStateCapella" => {
            decode_and_print(&bytes, BeaconStateCapella::<T>::from_ssz_bytes, format)?
        }
        other => return Err(format!("Unknown type: {}", other)),
    };

    Ok(())
}

fn decode_and_print<T: Serialize>(
    bytes: &[u8],
    decoder: impl FnOnce(&[u8]) -> Result<T, ssz::DecodeError>,
    output_format: OutputFormat,
) -> Result<(), String> {
    let item = decoder(bytes).map_err(|e| format!("SSZ decode failed: {e:?}"))?;

    match output_format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(&item)
                    .map_err(|e| format!("Unable to write object to JSON: {e:?}"))?
            );
        }
        OutputFormat::Yaml => {
            println!(
                "{}",
                serde_yaml::to_string(&item)
                    .map_err(|e| format!("Unable to write object to YAML: {e:?}"))?
            );
        }
    }

    Ok(())
}
