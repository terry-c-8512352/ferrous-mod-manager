// Builds a complete, self-contained fake $HOME so the whole app can run on a box
// with no Steam and no Paradox game installed.
//
// The real backend reads `dirs::home_dir()` (i.e. $HOME) and expects:
//   $HOME/.steam/steam/steamapps/libraryfolders.vdf
//   $HOME/.local/share/Paradox Interactive/<Game>/mod/*.mod
// where each `.mod` descriptor's `path=` points at that mod's content directory.
// Conflict + achievement detection walk that content dir, so the descriptors must
// carry *absolute* paths — which is exactly what a real install stores. We compute
// them at generation time after creating the dirs.
//
//   cargo run --example gen_mock_home -- ./mock-home   # build it
//   HOME=$(pwd)/mock-home cargo run --example mock_smoke -p ferrous-mod-manager
//   HOME=$(pwd)/mock-home cargo run -p app-ui   # real GUI (or: just mock-ui)
use std::fs;
use std::path::{Path, PathBuf};

/// One file inside a mod: (relative path under the mod's content dir, contents).
type ModFile = (&'static str, &'static str);

/// A mod to materialise: folder slug, display name, optional workshop id, files.
struct MockMod {
    slug: &'static str,
    name: &'static str,
    remote_file_id: Option<&'static str>,
    files: &'static [ModFile],
}

fn main() {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mock-home".to_string());
    let home = PathBuf::from(&target);

    if home.exists() {
        fs::remove_dir_all(&home).expect("clear existing mock home");
    }

    write_libraryfolders_vdf(&home);
    let mod_dir = home.join(".local/share/Paradox Interactive/Stellaris/mod");
    fs::create_dir_all(&mod_dir).expect("create mod dir");

    for m in mods() {
        write_mod(&mod_dir, m);
    }

    let abs = fs::canonicalize(&home).expect("canonicalize mock home");
    println!("Mock $HOME ready at: {}", abs.display());
    println!("\nTry it:");
    println!(
        "  HOME={} cargo run --example mock_smoke -p ferrous-mod-manager",
        abs.display()
    );
    println!("  HOME={} cargo run -p app-ui", abs.display());
}

/// Two Steam libraries: Stellaris (with mods) plus CK3 to exercise multi-game
/// detection. The `path` values are absolute and machine-independent on purpose —
/// detection only reads them to build display paths, it doesn't stat them.
fn write_libraryfolders_vdf(home: &Path) {
    let vdf = r#""libraryfolders"
{
	"0"
	{
		"path"		"/games/SteamLibrary"
		"label"		""
		"apps"
		{
			"281990"		"123456789"
			"1158310"		"123456789"
		}
	}
}
"#;
    let vdf_path = home.join(".steam/steam/steamapps");
    fs::create_dir_all(&vdf_path).expect("create steamapps dir");
    fs::write(vdf_path.join("libraryfolders.vdf"), vdf).expect("write vdf");
}

fn write_mod(mod_dir: &Path, m: MockMod) {
    let content_dir = mod_dir.join(m.slug);
    for (rel, body) in m.files {
        let file_path = content_dir.join(rel);
        fs::create_dir_all(file_path.parent().unwrap()).expect("create content subdir");
        fs::write(&file_path, body).expect("write mod file");
    }

    let abs_content = fs::canonicalize(&content_dir).expect("canonicalize content dir");

    let mut descriptor = format!("name=\"{}\"\n", m.name);
    descriptor.push_str("supported_version=\"v4.2.*\"\n");
    descriptor.push_str(&format!("path=\"{}\"\n", abs_content.display()));
    if let Some(id) = m.remote_file_id {
        descriptor.push_str(&format!("remote_file_id=\"{id}\"\n"));
    }

    fs::write(mod_dir.join(format!("{}.mod", m.slug)), descriptor).expect("write descriptor");
}

/// A spread of mods chosen to exercise every interesting code path:
///   * cosmetic-only mod that keeps achievements on
///   * gameplay mods that switch achievements off (events / common / defines)
///   * a deliberate `common/defines` conflict (high severity)
///   * a deliberate `localisation` conflict (low severity)
///   * a workshop mod (remote_file_id) keyed by Steam id rather than path
fn mods() -> Vec<MockMod> {
    vec![
        MockMod {
            slug: "ui_overhaul_dynamic",
            name: "UI Overhaul Dynamic",
            remote_file_id: None,
            files: &[
                ("interface/topbar.gui", "# tweaked topbar layout\n"),
                ("gfx/interface/icons/buttons.dds", "fake-dds-bytes\n"),
                (
                    "localisation/english/l_english.yml",
                    "l_english:\n msg_hello:0 \"Hi\"\n",
                ),
            ],
        },
        MockMod {
            slug: "more_events",
            name: "More Events Mod",
            remote_file_id: None,
            files: &[("events/extra_events.txt", "namespace = extra\n")],
        },
        MockMod {
            slug: "gigastructures",
            name: "Gigastructural Engineering",
            remote_file_id: None,
            files: &[
                ("common/defines/00_defines.txt", "NGameplay = { FOO = 1 }\n"),
                ("common/buildings/megastructures.txt", "ring_world = { }\n"),
                ("gfx/models/ringworld.mesh", "fake-mesh\n"),
            ],
        },
        MockMod {
            slug: "planetary_diversity",
            name: "Planetary Diversity",
            remote_file_id: None,
            files: &[
                // Conflicts with gigastructures on common/defines (Defines = high).
                ("common/defines/00_defines.txt", "NGameplay = { FOO = 2 }\n"),
                ("common/planet_classes/pd_classes.txt", "pc_gaia = { }\n"),
                // Conflicts with UI Overhaul on localisation (low severity).
                (
                    "localisation/english/l_english.yml",
                    "l_english:\n msg_hello:0 \"Hello\"\n",
                ),
            ],
        },
        MockMod {
            slug: "workshop_shipset",
            name: "Workshop Cosmetic Shipset",
            remote_file_id: Some("2890123456"),
            files: &[
                ("gfx/models/ships/cruiser.mesh", "fake-mesh\n"),
                ("sound/effects/engine.wav", "fake-wav\n"),
            ],
        },
    ]
}
