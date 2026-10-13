use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/notification.proto");

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    // Well-known types (`google/protobuf/struct.proto`) shipped with the vendored protoc.
    let well_known = protoc_bin_vendored::include_path()?;
    let mut prost_config = prost_build::Config::new();
    prost_config.protoc_executable(protoc);
    tonic_prost_build::configure().compile_with_config(
        prost_config,
        &[Path::new("proto/notification.proto")],
        &[Path::new("proto"), well_known.as_path()],
    )?;
    Ok(())
}
