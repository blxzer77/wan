use std::{error::Error, path::PathBuf};

use specta::Types;
use specta_typescript::Typescript;
use wan_types::{ApiError, ApiErrorData, ErrorKind, ProtocolVersion};

fn main() -> Result<(), Box<dyn Error>> {
    let types = Types::default()
        .register::<ApiError>()
        .register::<ApiErrorData>()
        .register::<ErrorKind>()
        .register::<ProtocolVersion>();
    let output =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/wan-protocol/src/index.ts");

    Typescript::default().export_to(output, &types, specta_serde::Format)?;
    Ok(())
}
