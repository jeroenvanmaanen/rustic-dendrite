fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/hello_world.proto")?;
    tonic_build::compile_protos("proto/grpc_example.proto")?;
    Ok(())
}