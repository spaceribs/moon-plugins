use proto_pdk_test_utils::*;

const BASE_URL: &str = "https://github.com/tilt-dev/tilt/releases/download/v0.37.8";

async fn download(os: HostOS, arch: HostArch, libc: HostLibc) -> DownloadPrebuiltOutput {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox
        .create_plugin_with_config("tilt-test", |config| {
            config.host_with(|host| {
                host.os = os;
                host.arch = arch;
                host.libc = libc;
            });
        })
        .await;

    plugin
        .download_prebuilt(DownloadPrebuiltInput {
            context: PluginContext {
                version: VersionSpec::parse("0.37.8").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        })
        .await
}

fn expected(file: &str) -> DownloadPrebuiltOutput {
    DownloadPrebuiltOutput {
        checksum_url: Some(format!("{BASE_URL}/checksums.txt")),
        download_name: Some(file.into()),
        download_url: format!("{BASE_URL}/{file}"),
        ..Default::default()
    }
}

mod tilt_tool {
    use super::*;

    // Downloads and installs a real release into the sandbox.
    generate_download_install_tests!("tilt-test", "0.37.8");

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        assert_eq!(
            download(HostOS::Linux, HostArch::X64, HostLibc::Gnu).await,
            expected("tilt.0.37.8.linux.x86_64.tar.gz")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        assert_eq!(
            download(HostOS::Linux, HostArch::Arm64, HostLibc::Gnu).await,
            expected("tilt.0.37.8.linux.arm64.tar.gz")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm() {
        assert_eq!(
            download(HostOS::Linux, HostArch::Arm, HostLibc::Gnu).await,
            expected("tilt.0.37.8.linux.arm.tar.gz")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_alpine_x64() {
        assert_eq!(
            download(HostOS::Linux, HostArch::X64, HostLibc::Musl).await,
            expected("tilt.0.37.8.linux-alpine.x86_64.tar.gz")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_alpine_arm64() {
        assert_eq!(
            download(HostOS::Linux, HostArch::Arm64, HostLibc::Musl).await,
            expected("tilt.0.37.8.linux-alpine.arm64.tar.gz")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Unable to install Tilt")]
    async fn doesnt_support_alpine_arm() {
        download(HostOS::Linux, HostArch::Arm, HostLibc::Musl).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        assert_eq!(
            download(HostOS::MacOS, HostArch::X64, HostLibc::Unknown).await,
            expected("tilt.0.37.8.mac.x86_64.tar.gz")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        assert_eq!(
            download(HostOS::MacOS, HostArch::Arm64, HostLibc::Unknown).await,
            expected("tilt.0.37.8.mac.arm64.tar.gz")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        assert_eq!(
            download(HostOS::Windows, HostArch::X64, HostLibc::Unknown).await,
            expected("tilt.0.37.8.windows.x86_64.zip")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Unable to install Tilt")]
    async fn doesnt_support_windows_arm64() {
        download(HostOS::Windows, HostArch::Arm64, HostLibc::Unknown).await;
    }
}
