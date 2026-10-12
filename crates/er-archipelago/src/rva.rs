use pelite::pe64::{Pe, PeView};
use std::sync::LazyLock;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::core::PCSTR;

mod bundle;
mod rva_ww;

pub use bundle::RvaBundle;

const NAME: &str = "ELDEN RING™";
const WW_VERSION: &str = "2.7.1.0";
const LANG_ID_EN: u16 = 0x0009;
const JP_VERSION: &str = "2.7.1.1";
const LANG_ID_JP: u16 = 0x0011;

/// Returns the RVA bundle for the current executable region and version.
///
/// This will panic if the current executable isn't supported by this package.
pub fn get() -> &'static RvaBundle {
    static RVAS: LazyLock<RvaBundle> = LazyLock::new(|| {
        let module = unsafe {
            PeView::module(GetModuleHandleA(PCSTR(std::ptr::null())).unwrap().0 as *const u8)
        };
        detect_version_and_get_rvas(&module)
    });

    &RVAS
}

/// Determines the region and version of the current executable and returns the
/// [RvaBundle] for it. Panics if the version isn't known.
/// This is derived from (and duplicates) bundle extraction in fromsoftware-rs.
fn detect_version_and_get_rvas(module: &PeView) -> RvaBundle {
    let resources = module.resources().unwrap();
    let info = resources.version_info().unwrap();

    // Extract version info
    let product_version = info
        .fixed()
        .expect("Executable doesn't contain version metdata")
        .dwProductVersion;
    let version = format!(
        "{}.{}.{}.{}",
        product_version.Major, product_version.Minor, product_version.Patch, product_version.Build,
    );

    // Extract product name
    let language = *info
        .translation()
        .first()
        .expect("Executable doesn't contain language metdata");
    let mut product_name: Option<String> = None;
    info.strings(language, |k, v| {
        if k == "ProductName" {
            product_name = Some(v.to_string());
        }
    });

    let product = product_name.expect("Executable doesn't contain product name metadata");
    if product != NAME {
        panic!(
            "Expected executable name to be \"{}\", was \"{}\"",
            NAME, &product
        );
    }

    // English 2.7.1.0 or Japanese 2.7.1.1. Every RVA in this bundle is a function, and upstream
    // fromsoftware-rs's generated WW 2.7.1.0 and JP 2.7.1.1 tables agree on all function
    // addresses (they differ only in vtables), so Japanese shares the WW table.
    let lang_id_base = language.lang_id & 0x03FF;
    let expected = match lang_id_base {
        LANG_ID_EN => WW_VERSION,
        LANG_ID_JP => JP_VERSION,
        _ => panic!(
            "Expected executable language ID to be {:#04x} or {:#04x}, was {:#04x}",
            LANG_ID_EN, LANG_ID_JP, lang_id_base
        ),
    };

    if version != expected {
        panic!(
            "Expected executable version \"{}\", was \"{}\"",
            expected, &version
        );
    }

    if lang_id_base == LANG_ID_JP {
        log::warn!("Japanese executable {version}: using Worldwide 2.7.1.0 function addresses");
    }

    rva_ww::RVAS
}
