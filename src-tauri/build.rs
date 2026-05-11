fn main() {
    compile_shared_protos();
    tauri_build::build()
}

fn compile_shared_protos() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    let proto_dir = std::path::Path::new(&manifest_dir).join("../../docs/proto");
    if !proto_dir.is_dir() {
        panic!("shared protobuf directory missing: {}", proto_dir.display());
    }

    let protos = [
        "common.proto",
        "records.proto",
        "events.proto",
        "resources.proto",
        "control.proto",
        "consensus.proto",
        "skin_station.proto",
    ];
    let proto_paths: Vec<_> = protos.iter().map(|name| proto_dir.join(name)).collect();

    println!("cargo:rerun-if-changed={}", proto_dir.display());
    for proto in &proto_paths {
        println!("cargo:rerun-if-changed={}", proto.display());
    }

    let mut config = prost_build::Config::new();
    config
        .type_attribute(".", "#[allow(dead_code)]")
        .compile_protos(&proto_paths, &[proto_dir])
        .expect("failed to compile shared protobuf definitions");
}
