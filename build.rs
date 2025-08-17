fn main() {
    // See https://docs.rs/protobuf-codegen/latest/protobuf_codegen/#how-to-generate-code
    println!("cargo::rerun-if-changed=proto/ais/v1/spec.proto");
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .includes(&["proto"])
        .input("proto/ais/v1/spec.proto")
        .cargo_out_dir("proto_generated")
        .run_from_script();
}
