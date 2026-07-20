fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().
    compile_protos(
        &["../../packages/proto/simulation.proto"], 
        &["../../packages/proto"])?;
    Ok(())
}