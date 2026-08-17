fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile_protos(
        &["../../packages/proto/simulation.proto"],
        &["../../packages/proto"],
    )?;

    println!("cargo:rerun-if-changed=../../VERSION");
    let version = std::fs::read_to_string("../../VERSION")?;
    println!("cargo:rustc-env=APP_VERSION={}", version.trim());

    Ok(())
}
