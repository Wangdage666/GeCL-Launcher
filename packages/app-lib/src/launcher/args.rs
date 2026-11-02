//! Minecraft CLI argument logic
use crate::instance::QuickPlayType;
use crate::launcher::quick_play_version::QuickPlayServerVersion;
use crate::launcher::{QuickPlayVersion, parse_rules};
use crate::state::Credentials;
use crate::{
    state::{MemorySettings, WindowSize},
    util::{io::IOError, platform::classpath_separator},
};
use anyhow::Context;
use daedalus::minecraft::LoggingConfiguration;
use daedalus::{
    get_path_from_artifact,
    minecraft::{Argument, ArgumentValue, Library, VersionType},
    modded::SidedDataEntry,
};
use dunce::canonicalize;
use itertools::Itertools;
use std::io::{BufRead, BufReader, ErrorKind};
use std::net::SocketAddr;
use std::{collections::HashMap, path::Path};
use uuid::Uuid;

// Replaces the space separator with a newline character, as to not split the arguments
const TEMPORARY_REPLACE_CHAR: &str = "\n";

pub fn get_class_paths(
    libraries_path: &Path,
    libraries: &[Library],
    launcher_class_path: &[&Path],
    java_arch: &str,
    minecraft_updated: bool,
) -> crate::Result<String> {
    launcher_class_path
        .iter()
        .map(|path| {
            Ok(canonicalize(path)
                .context(crate::ErrorKind::LauncherError(format!(
                    "specified class path {} does not exist",
                    path.display()
                )))?
                .to_string_lossy()
                .to_string())
        })
        .chain(libraries.iter().filter_map(|library| {
            if let Some(rules) = &library.rules
                && !parse_rules(
                    rules,
                    java_arch,
                    &QuickPlayType::None,
                    minecraft_updated,
                )
            {
                return None;
            }

            if !library.include_in_classpath {
                return None;
            }

            Some(get_lib_path(
                libraries_path,
                &library.name,
                library.natives_os_key_and_classifiers(java_arch).is_some(),
            ))
        }))
        .process_results(|iter| {
            iter.unique().join(classpath_separator(java_arch))
        })
}

pub fn get_class_paths_jar<T: AsRef<str>>(
    libraries_path: &Path,
    libraries: &[T],
    java_arch: &str,
) -> crate::Result<String> {
    let cps = libraries
        .iter()
        .map(|library| get_lib_path(libraries_path, library.as_ref(), false))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(cps.join(classpath_separator(java_arch)))
}

pub fn get_lib_path(
    libraries_path: &Path,
    lib: &str,
    allow_not_exist: bool,
) -> crate::Result<String> {
    let path = libraries_path.join(get_path_from_artifact(lib)?);

    let path = match canonicalize(&path) {
        Ok(p) => p,
        Err(err) if err.kind() == ErrorKind::NotFound && allow_not_exist => {
            path
        }
        Err(err) => {
            return Err(err).context(crate::ErrorKind::LauncherError(format!(
                "could not canonicalize library path {}",
                path.display()
            )));
        }
    };

    Ok(java_path(&path))
}

pub(super) fn java_path(path: &Path) -> String {
    dunce::simplified(path).to_string_lossy().into_owned()
}

fn log_config_location(path: &Path) -> String {
    let path = dunce::simplified(path);
    #[cfg(windows)]
    if matches!(path.components().next(), Some(std::path::Component::Prefix(prefix)) if prefix.kind().is_verbatim())
    {
        // Preserve the semantics of long/reserved/special paths.
        return java_path(path);
    }
    url::Url::from_file_path(path)
        .map(|uri| uri.to_string().replace(',', "%2C"))
        .unwrap_or_else(|_| java_path(path))
}

#[allow(clippy::too_many_arguments)]
pub fn get_jvm_arguments(
    arguments: Option<&[Argument]>,
    natives_path: &Path,
    libraries_path: &Path,
    log_configs_path: &Path,
    class_paths: &str,
    agent_path: &Path,
    version_name: &str,
    memory: MemorySettings,
    custom_args: Vec<String>,
    java_arch: &str,
    quick_play_type: &QuickPlayType,
    quick_play_version: QuickPlayVersion,
    log_config: Option<&LoggingConfiguration>,
    ipc_addr: SocketAddr,
) -> crate::Result<Vec<String>> {
    let mut parsed_arguments = Vec::new();

    if let Some(args) = arguments {
        parse_arguments(
            args,
            &mut parsed_arguments,
            |arg| {
                parse_jvm_argument(
                    arg.to_string(),
                    natives_path,
                    libraries_path,
                    class_paths,
                    version_name,
                    java_arch,
                )
            },
            java_arch,
            quick_play_type,
        )?;
    } else {
        parsed_arguments.push(format!(
            "-Djava.library.path={}",
            canonicalize(natives_path)
                .context(crate::ErrorKind::LauncherError(format!(
                    "specified natives path {} does not exist",
                    natives_path.display()
                )))?
                .to_string_lossy()
        ));
        parsed_arguments.push("-cp".to_string());
        parsed_arguments.push(class_paths.to_string());
    }

    parsed_arguments.push(format!("-Xmx{}M", memory.maximum));

    if let Some(LoggingConfiguration::Log4j2Xml { argument, file }) = log_config
    {
        let full_path = log_configs_path.join(&file.id);
        // Older Log4j also needs URI encoding for spaces and non-ASCII names.
        let full_path = log_config_location(&full_path);
        parsed_arguments.push(argument.replace("${path}", &full_path));
    }

    parsed_arguments.push(format!(
        "-javaagent:{}",
        canonicalize(agent_path)
            .context(crate::ErrorKind::LauncherError(format!(
                "specified java agent path {} does not exist",
                agent_path.display()
            )))?
            .to_string_lossy()
    ));

    parsed_arguments
        .push(format!("-Dmodrinth.internal.ipc.host={}", ipc_addr.ip()));
    parsed_arguments
        .push(format!("-Dmodrinth.internal.ipc.port={}", ipc_addr.port()));

    parsed_arguments.push(format!(
        "-Dmodrinth.internal.quickPlay.serverVersion={}",
        serde_json::to_value(quick_play_version.server)?
            .as_str()
            .unwrap()
    ));
    if let QuickPlayType::Server(server) = quick_play_type
        && quick_play_version.server == QuickPlayServerVersion::Injected
    {
        let (host, port) = server.require_resolved()?;
        parsed_arguments.extend_from_slice(&[
            format!("-Dmodrinth.internal.quickPlay.host={host}"),
            format!("-Dmodrinth.internal.quickPlay.port={port}"),
        ]);
    }

    for arg in custom_args {
        if !arg.is_empty() {
            parsed_arguments.push(arg);
        }
    }

    Ok(parsed_arguments)
}

fn parse_jvm_argument(
    mut argument: String,
    natives_path: &Path,
    libraries_path: &Path,
    class_paths: &str,
    version_name: &str,
    java_arch: &str,
) -> crate::Result<String> {
    argument.retain(|c| !c.is_whitespace());
    Ok(argument
        .replace(
            "${natives_directory}",
            &canonicalize(natives_path)
                .context(crate::ErrorKind::LauncherError(format!(
                    "specified natives path {} does not exist",
                    natives_path.display()
                )))?
                .to_string_lossy(),
        )
        .replace(
            "${library_directory}",
            &canonicalize(libraries_path)
                .context(crate::ErrorKind::LauncherError(format!(
                    "specified libraries path {} does not exist",
                    libraries_path.display()
                )))?
                .to_string_lossy(),
        )
        .replace("${classpath_separator}", classpath_separator(java_arch))
        .replace("${launcher_name}", "GeCL")
        .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
        .replace("${version_name}", version_name)
        .replace("${classpath}", class_paths))
}

#[allow(clippy::too_many_arguments)]
pub async fn get_minecraft_arguments(
    arguments: Option<&[Argument]>,
    legacy_arguments: Option<&str>,
    credentials: &Credentials,
    version: &str,
    asset_index_name: &str,
    game_directory: &Path,
    assets_directory: &Path,
    version_type: &VersionType,
    resolution: WindowSize,
    java_arch: &str,
    quick_play_type: &QuickPlayType,
    quick_play_version: QuickPlayVersion,
) -> crate::Result<Vec<String>> {
    let access_token = credentials.access_token.clone();
    let user_type =
        if credentials.account_type == crate::state::AccountType::Offline {
            "legacy"
        } else {
            "msa"
        };
    let profile = credentials.maybe_online_profile().await;
    let mut parsed_arguments = Vec::new();

    if let Some(arguments) = arguments {
        parse_arguments(
            arguments,
            &mut parsed_arguments,
            |arg| {
                parse_minecraft_argument(
                    arg,
                    &access_token,
                    user_type,
                    &profile.name,
                    profile.id,
                    version,
                    asset_index_name,
                    game_directory,
                    assets_directory,
                    version_type,
                    resolution,
                    quick_play_type,
                )
            },
            java_arch,
            quick_play_type,
        )?;
    } else if let Some(legacy_arguments) = legacy_arguments {
        for x in legacy_arguments.split(' ') {
            parsed_arguments.push(parse_minecraft_argument(
                &x.replace(' ', TEMPORARY_REPLACE_CHAR),
                &access_token,
                user_type,
                &profile.name,
                profile.id,
                version,
                asset_index_name,
                game_directory,
                assets_directory,
                version_type,
                resolution,
                quick_play_type,
            )?);
        }
    }

    if let QuickPlayType::Server(server) = quick_play_type
        && quick_play_version.server == QuickPlayServerVersion::BuiltinLegacy
    {
        let (host, port) = server.require_resolved()?;
        parsed_arguments.extend_from_slice(&[
            "--server".to_string(),
            host.to_string(),
            "--port".to_string(),
            port.to_string(),
        ]);
    }

    Ok(parsed_arguments)
}

#[allow(clippy::too_many_arguments)]
fn parse_minecraft_argument(
    argument: &str,
    access_token: &str,
    user_type: &str,
    username: &str,
    uuid: Uuid,
    version: &str,
    asset_index_name: &str,
    game_directory: &Path,
    assets_directory: &Path,
    _version_type: &VersionType,
    resolution: WindowSize,
    quick_play_type: &QuickPlayType,
) -> crate::Result<String> {
    Ok(argument
        .replace("${accessToken}", access_token)
        .replace("${auth_access_token}", access_token)
        .replace("${auth_session}", access_token)
        .replace("${auth_player_name}", username)
        // TODO: add auth xuid eventually
        .replace("${auth_xuid}", "0")
        .replace("${auth_uuid}", &uuid.simple().to_string())
        .replace("${uuid}", &uuid.simple().to_string())
        .replace("${clientid}", crate::state::MICROSOFT_CLIENT_ID)
        .replace("${user_properties}", "{}")
        .replace("${user_type}", user_type)
        .replace("${version_name}", version)
        .replace("${assets_index_name}", asset_index_name)
        .replace(
            "${game_directory}",
            &canonicalize(game_directory)
                .context(crate::ErrorKind::LauncherError(format!(
                    "specified game directory {} does not exist",
                    game_directory.display()
                )))?
                .to_string_lossy(),
        )
        .replace(
            "${assets_root}",
            &canonicalize(assets_directory)
                .context(crate::ErrorKind::LauncherError(format!(
                    "specified assets directory {} does not exist",
                    assets_directory.display()
                )))?
                .to_string_lossy(),
        )
        .replace(
            "${game_assets}",
            &canonicalize(assets_directory)
                .context(crate::ErrorKind::LauncherError(format!(
                    "specified assets directory {} does not exist",
                    assets_directory.display()
                )))?
                .to_string_lossy(),
        )
        .replace("${version_type}", "started by GeCL")
        .replace("${resolution_width}", &resolution.0.to_string())
        .replace("${resolution_height}", &resolution.1.to_string())
        .replace(
            "${quickPlaySingleplayer}",
            match quick_play_type {
                QuickPlayType::Singleplayer(world) => world,
                _ => "",
            },
        )
        .replace(
            "${quickPlayMultiplayer}",
            &match quick_play_type {
                QuickPlayType::Server(address) => address.to_string(),
                _ => "".to_string(),
            },
        ))
}

fn parse_arguments<F>(
    arguments: &[Argument],
    parsed_arguments: &mut Vec<String>,
    parse_function: F,
    java_arch: &str,
    quick_play_type: &QuickPlayType,
) -> crate::Result<()>
where
    F: Fn(&str) -> crate::Result<String>,
{
    for argument in arguments {
        match argument {
            Argument::Normal(arg) => {
                let parsed =
                    parse_function(&arg.replace(' ', TEMPORARY_REPLACE_CHAR))?;
                for arg in parsed.split(TEMPORARY_REPLACE_CHAR) {
                    parsed_arguments.push(arg.to_string());
                }
            }
            Argument::Ruled { rules, value } => {
                if parse_rules(rules, java_arch, quick_play_type, true) {
                    match value {
                        ArgumentValue::Single(arg) => {
                            parsed_arguments.push(parse_function(
                                &arg.replace(' ', TEMPORARY_REPLACE_CHAR),
                            )?);
                        }
                        ArgumentValue::Many(args) => {
                            for arg in args {
                                parsed_arguments.push(parse_function(
                                    &arg.replace(' ', TEMPORARY_REPLACE_CHAR),
                                )?);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn get_processor_arguments(
    libraries_path: &Path,
    arguments: &[impl AsRef<str>],
    data: &HashMap<String, SidedDataEntry>,
) -> crate::Result<Vec<String>> {
    // We use iterator combinators to make sure that 1 input argument maps
    // to exactly 1 output argument. Otherwise you might get issues that take
    // days to debug *sigh*
    //
    // Arguments can be enclosed in square brackets [] if they are not taken
    // literally, but are actually resolved to the path of a library we
    // previously downloaded.
    // For example, `[net.neoforged:neoform:1.21.10-20251010.172816:mappings@tsrg.lzma]`.
    //
    // Otherwise, arguments may contain `{KEY}` variable placeholders, which
    // must be replaced with the corresponding value from `data`.
    // Importantly, variables might not *just* be `{KEY}`, but may also be
    // e.g. `{KEY}/some more values`. For example, `{ROOT}/libraries/`.
    // Therefore, it is important that we don't just check if the variable is
    // enclosed in `{}`s, but actually do a find-and-replace with all variables.
    //
    // Currently, we do it in a naive way where we iterate over every `data`
    // entry and just `.replace()`, which is not efficient, but we shouldn't
    // have a lot of entries in `data`, and this code is not run often anyway.

    arguments
        .iter()
        .map(|arg| {
            let arg = arg.as_ref();
            if let Some(arg) = arg.strip_prefix('[')
                && let Some(lib_key) = arg.strip_suffix(']')
            {
                // this should resolve to the path of a library
                get_lib_path(libraries_path, lib_key, true)
            } else {
                let mut arg = arg.to_string();

                // replace variables like `{PATH}` to their real values
                for (key, entry) in data {
                    let replacement = if let Some(arg) =
                        entry.client.strip_prefix('[')
                        && let Some(lib_key) = arg.strip_suffix(']')
                    {
                        // if the value of `PATH` in `data` is also a library key,
                        // it'll be enclosed in `[]`s, and we resolve it to a real lib path
                        get_lib_path(libraries_path, lib_key, true)?
                    } else {
                        // otherwise we just take the value in `data` literally
                        entry.client.clone()
                    };

                    arg = arg.replace(&format!("{{{key}}}"), &replacement);
                }

                Ok(arg)
            }
        })
        .collect::<crate::Result<Vec<_>>>()
}

pub async fn get_processor_main_class(
    path: String,
) -> crate::Result<Option<String>> {
    let main_class = tokio::task::spawn_blocking(move || {
        let zipfile = std::fs::File::open(&path)
            .map_err(|e| IOError::with_path(e, &path))?;
        let mut archive = zip::ZipArchive::new(zipfile).context(
            crate::ErrorKind::LauncherError(format!(
                "cannot read processor at {path}"
            )),
        )?;

        let file = archive.by_name("META-INF/MANIFEST.MF").context(
            crate::ErrorKind::LauncherError(format!(
                "cannot read processor manifest at {path}"
            )),
        )?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let mut line = line.map_err(IOError::from)?;
            line.retain(|c| !c.is_whitespace());

            if line.starts_with("Main-Class:")
                && let Some(class) = line.split(':').nth(1)
            {
                return Ok(Some(class.to_string()));
            }
        }

        Ok::<Option<String>, crate::Error>(None)
    })
    .await??;

    Ok(main_class)
}

#[cfg(all(test, windows))]
mod processor_path_tests {
    use super::*;

    #[test]
    fn launch_branding_keeps_version_identity_and_multiword_value() {
        let temp = tempfile::tempdir().unwrap();
        for version_type in [VersionType::Release, VersionType::Snapshot] {
            let parse = |arg: &str| {
                parse_minecraft_argument(
                    arg,
                    "fixture-token",
                    "legacy",
                    "Fixture",
                    Uuid::nil(),
                    "1.20.1",
                    "5",
                    temp.path(),
                    temp.path(),
                    &version_type,
                    WindowSize(854, 480),
                    &QuickPlayType::None,
                )
            };
            let modern = [
                "--version",
                "${version_name}",
                "--versionType",
                "${version_type}",
            ]
            .map(|s| Argument::Normal(s.to_owned()));
            let mut arguments = Vec::new();
            parse_arguments(
                &modern,
                &mut arguments,
                parse,
                "amd64",
                &QuickPlayType::None,
            )
            .unwrap();
            assert_eq!(
                arguments,
                ["--version", "1.20.1", "--versionType", "started by GeCL"]
            );

            let legacy: Vec<_> =
                "--version ${version_name} --versionType ${version_type}"
                    .split(' ')
                    .map(|arg| parse(arg).unwrap())
                    .collect();
            assert_eq!(legacy, arguments);
            let mut unsupported = Vec::new();
            parse_arguments(
                &modern[..2],
                &mut unsupported,
                parse,
                "amd64",
                &QuickPlayType::None,
            )
            .unwrap();
            assert_eq!(unsupported, ["--version", "1.20.1"]);
        }
        let brand = parse_jvm_argument(
            "-Dminecraft.launcher.brand=${launcher_name}".into(),
            temp.path(),
            temp.path(),
            "fixture-classpath",
            "1.20.1",
            "amd64",
        )
        .unwrap();
        assert_eq!(brand, "-Dminecraft.launcher.brand=GeCL");
    }

    #[test]
    fn jvm_logging_configuration_uses_a_single_java_compatible_path() {
        use crate::launcher::quick_play_version::QuickPlaySingleplayerVersion;

        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path())
            .unwrap()
            .join("GeCL (x86) 中文");
        let natives = root.join("natives");
        let libraries = root.join("libraries");
        let logs = root.join("log configs #,%");
        for directory in [&natives, &libraries, &logs] {
            std::fs::create_dir_all(directory).unwrap();
        }
        let agent = root.join("agent.jar");
        std::fs::write(&agent, b"fixture").unwrap();
        let config_path = logs.join("client-1.12.xml");
        std::fs::write(&config_path, b"<Configuration/>").unwrap();
        let config: LoggingConfiguration = serde_json::from_value(serde_json::json!({
            "type": "log4j2-xml",
            "argument": "-Dlog4j.configurationFile=${path}",
            "file": {"id":"client-1.12.xml", "sha1":"fixture", "size":16, "url":"https://example.invalid/config.xml"}
        })).unwrap();
        for template in [None, Some([].as_slice())] {
            for logging in [None, Some(&config)] {
                let arguments = get_jvm_arguments(
                    template,
                    &natives,
                    &libraries,
                    &logs,
                    "fixture classpath",
                    &agent,
                    "1.20.1",
                    MemorySettings { maximum: 1024 },
                    Vec::new(),
                    "amd64",
                    &QuickPlayType::None,
                    QuickPlayVersion {
                        server: QuickPlayServerVersion::Unsupported,
                        singleplayer: QuickPlaySingleplayerVersion::Unsupported,
                    },
                    logging,
                    "127.0.0.1:1".parse().unwrap(),
                )
                .unwrap();
                let properties: Vec<_> = arguments
                    .iter()
                    .filter(|arg| arg.starts_with("-Dlog4j.configurationFile="))
                    .collect();
                if logging.is_some() {
                    assert_eq!(properties.len(), 1);
                    assert_eq!(
                        *properties[0],
                        format!(
                            "-Dlog4j.configurationFile={}",
                            log_config_location(&config_path)
                        )
                    );
                    assert!(!properties[0].contains(r"\\?\"));
                    let location = properties[0]
                        .strip_prefix("-Dlog4j.configurationFile=")
                        .unwrap();
                    assert_eq!(
                        url::Url::parse(location)
                            .unwrap()
                            .to_file_path()
                            .unwrap(),
                        Path::new(&java_path(&config_path))
                    );
                    assert!(location.contains("%20"));
                    assert!(location.contains("%E4%B8%AD%E6%96%87"));
                    assert!(location.contains("%23%2C%25"));
                } else {
                    assert!(properties.is_empty());
                }
            }
        }
    }

    #[test]
    fn uncreated_processor_outputs_use_java_compatible_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path())
            .unwrap()
            .join("GeCL (x86) 中文");
        let libraries = root.join("meta/libraries");
        let artifact =
            "net.minecraft:client:1.20.1-20230612.114412:mappings@txt";
        let data = HashMap::from([
            (
                "MOJMAPS".to_owned(),
                SidedDataEntry {
                    client: format!("[{artifact}]"),
                    server: String::new(),
                },
            ),
            (
                "ROOT".to_owned(),
                SidedDataEntry {
                    client: java_path(&root),
                    server: String::new(),
                },
            ),
        ]);
        let arguments = get_processor_arguments(
            &libraries,
            &["--output", "{MOJMAPS}", "{ROOT}/run args.txt"],
            &data,
        )
        .unwrap();
        let output = libraries.join(get_path_from_artifact(artifact).unwrap());

        assert_eq!(arguments.len(), 3);
        assert_eq!(arguments[0], "--output");
        assert_eq!(arguments[1], java_path(&output));
        assert!(!arguments[1].starts_with(r"\\?\"));
        assert_eq!(arguments[2], format!("{}/run args.txt", java_path(&root)));
        assert!(arguments[1].contains("GeCL (x86) 中文"));
        assert!(!output.exists());
        assert!(get_lib_path(&libraries, artifact, false).is_err());
    }

    #[test]
    fn java_paths_preserve_special_windows_path_semantics() {
        let long = format!(r"\\?\C:\{}\output.txt", "directory\\".repeat(35));
        for path in [
            r"\\server\share\Game Folder\output.txt",
            r"\\?\UNC\server\share\output.txt",
            r"\\?\C:\Game\NUL",
            r"\\?\C:\Game\trailing.\output.txt",
            &long,
        ] {
            assert_eq!(java_path(Path::new(path)), path);
        }
        assert_eq!(
            java_path(Path::new(r"C:\Game Folder\中文\output.txt")),
            r"C:\Game Folder\中文\output.txt"
        );
    }
}
