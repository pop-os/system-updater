use async_fs as fs;

const SOURCES_LIST: &str = "/etc/apt/sources.list";
const SYSTEM_SOURCES: &str = "/etc/apt/sources.list.d/system.sources";
const PROPRIETARY_SOURCES: &str = "/etc/apt/sources.list.d/pop-os-apps.sources";
const RELEASE_SOURCES: &str = "/etc/apt/sources.list.d/pop-os-ppa.sources";

const SOURCES_LIST_PLACEHOLDER: &str = r#"## This file is deprecated in Pop!_OS.
## See `man deb822` and /etc/apt/sources.list.d/system.sources.
"#;

pub async fn regenerate(release: &str) -> anyhow::Result<()> {
    if release == "impish" {
        futures::try_join!(
            fs::write(SOURCES_LIST, SOURCES_LIST_PLACEHOLDER),
            fs::write(SYSTEM_SOURCES, system_sources(release)),
            fs::write(PROPRIETARY_SOURCES, proprietary_sources(release)),
            fs::write(RELEASE_SOURCES, release_sources(release)),
        )?;
    }

    Ok(())
}

fn system_sources(release: &str) -> String {
    format!(
        r#"X-Repolib-Name: Pop_OS System Sources
Enabled: yes
Types: deb deb-src
URIs: http://us.archive.ubuntu.com/ubuntu/
Suites: {0} {0}-security {0}-updates {0}-backports
Components: main restricted universe multiverse
X-Repolib-Default-Mirror: http://us.archive.ubuntu.com/ubuntu/
"#,
        release
    )
}

fn proprietary_sources(release: &str) -> String {
    format!(
        r#"X-Repolib-Name: Pop_OS Apps
Enabled: yes
Types: deb
URIs: http://apt.pop-os.org/proprietary
Suites: {0}
Components: main
"#,
        release
    )
}

fn release_sources(release: &str) -> String {
    format!(
        r#"X-Repolib-Name: Pop_OS Release Sources
Enabled: yes
Types: deb deb-src
URIs: http://apt.pop-os.org/release
Suites: {0}
Components: main
"#,
        release
    )
}
