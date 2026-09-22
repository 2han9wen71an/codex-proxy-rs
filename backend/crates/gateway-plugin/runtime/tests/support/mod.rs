use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write as _,
    sync::Arc,
};

use flate2::{Compression, write::GzEncoder};
use gateway_plugin_sdk::{
    Capability, ContributionDeclaration, Contributions, Engines, Manifest, PROTOCOL_VERSION,
    Package, PackageTarget, RuntimeKind, Stage,
};
use sha2::{Digest as _, Sha256};
pub mod diagnostics;
pub mod environment;
pub mod http;
pub mod store;

pub const DEFAULT_PLUGIN_ID: &str = "test.example";

pub fn contribution(
    capability: Capability,
    stages: Vec<Stage>,
    input_formats: Vec<String>,
    output_formats: Vec<String>,
) -> (Capability, ContributionDeclaration) {
    contribution_for_id(
        DEFAULT_PLUGIN_ID,
        capability,
        stages,
        input_formats,
        output_formats,
    )
}

pub fn contribution_for_id(
    plugin_id: &str,
    capability: Capability,
    stages: Vec<Stage>,
    input_formats: Vec<String>,
    output_formats: Vec<String>,
) -> (Capability, ContributionDeclaration) {
    let local_id = match capability {
        Capability::Models => "models",
        Capability::Authentication => "authentication",
        Capability::FrontendAuthentication => "frontendAuthentication",
        Capability::Scheduler => "scheduler",
        Capability::ModelRouter => "modelRouter",
        Capability::Executor => "executor",
        Capability::Middleware => "middleware",
        Capability::RequestLifecycle => "requestLifecycle",
        Capability::WebSocketObserver => "webSocketObserver",
        Capability::Usage => "usage",
        Capability::CommandLine => "commandLine",
        Capability::Management => "management",
        Capability::Quota => "quota",
        Capability::RequestProfile => "requestProfile",
        Capability::AccountManagement => "accountManagement",
        Capability::Billing => "billing",
        Capability::Maintenance => "maintenance",
    };
    (
        capability,
        ContributionDeclaration {
            id: format!("{plugin_id}.{local_id}"),
            version: 1,
            stages,
            input_formats,
            output_formats,
        },
    )
}

pub fn worker() -> &'static [u8] {
    static WORKER: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    WORKER.get_or_init(|| {
        std::fs::read(env!("CARGO_BIN_EXE_gateway-plugin-test-worker"))
            .expect("Cargo 应为集成测试构建 Rust 测试插件")
    })
}

pub fn package(worker: &[u8]) -> Arc<[u8]> {
    package_with_permissions(worker, Vec::new())
}

pub fn package_with_permissions(
    worker: &[u8],
    permissions: Vec<gateway_plugin_sdk::Permission>,
) -> Arc<[u8]> {
    package_with_contributions(worker, permissions, Contributions::new())
}

pub fn package_with_contributions(
    worker: &[u8],
    permissions: Vec<gateway_plugin_sdk::Permission>,
    contributes: Contributions,
) -> Arc<[u8]> {
    package_with_contributions_and_state(worker, permissions, contributes, vec![])
}

pub fn package_with_contributions_for_id(
    worker: &[u8],
    plugin_id: &str,
    permissions: Vec<gateway_plugin_sdk::Permission>,
    contributes: Contributions,
) -> Arc<[u8]> {
    package_with_identity_and_state(worker, plugin_id, permissions, contributes, vec![])
}

pub fn package_with_contributions_and_state(
    worker: &[u8],
    permissions: Vec<gateway_plugin_sdk::Permission>,
    contributes: Contributions,
    state: Vec<gateway_plugin_sdk::StateNamespace>,
) -> Arc<[u8]> {
    package_with_identity_and_state(worker, DEFAULT_PLUGIN_ID, permissions, contributes, state)
}

fn package_with_identity_and_state(
    worker: &[u8],
    plugin_id: &str,
    permissions: Vec<gateway_plugin_sdk::Permission>,
    contributes: Contributions,
    state: Vec<gateway_plugin_sdk::StateNamespace>,
) -> Arc<[u8]> {
    let (publisher, name) = plugin_id
        .split_once('.')
        .expect("测试插件 ID 必须使用 publisher.name");
    let files = BTreeMap::from([("bin/worker".to_owned(), hex::encode(Sha256::digest(worker)))]);
    let manifest = Manifest {
        manifest_version: gateway_plugin_sdk::MANIFEST_VERSION,
        name: name.to_owned(),
        display_name: "示例".to_owned(),
        publisher: publisher.to_owned(),
        version: "1.0.0".parse().unwrap(),
        description: "测试插件".to_owned(),
        license: "MIT".to_owned(),
        author: Some("test".to_owned()),
        engines: Engines {
            codex_proxy_rs: ">=1.0.0, <2.0.0".parse().unwrap(),
        },
        main: "bin/worker".to_owned(),
        runtime: RuntimeKind::TrustedProcess,
        contributes,
        permissions: permissions.into_iter().collect::<BTreeSet<_>>(),
        configuration_schema: serde_json::json!({}),
        secret_fields: BTreeSet::new(),
        resources: BTreeMap::new(),
        icon: None,
        state,
        package: Some(Package {
            protocol_version: PROTOCOL_VERSION,
            target: PackageTarget {
                os: std::env::consts::OS.to_owned(),
                architecture: std::env::consts::ARCH.to_owned(),
            },
            files,
        }),
    };
    archive(BTreeMap::from([
        ("plugin.json".into(), serde_json::to_vec(&manifest).unwrap()),
        ("bin/worker".into(), worker.to_vec()),
    ]))
}

pub fn archive(files: BTreeMap<String, Vec<u8>>) -> Arc<[u8]> {
    let mut tar = tar::Builder::new(Vec::new());
    for (path, bytes) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o700);
        header.set_cksum();
        tar.append_data(&mut header, path, bytes.as_slice())
            .unwrap();
    }
    let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
    gzip.write_all(&tar.into_inner().unwrap()).unwrap();
    gzip.finish().unwrap().into()
}
