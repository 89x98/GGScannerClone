//! Reveal an exported file using the native Windows shell.

#[cfg(target_os = "windows")]
pub fn reveal_file(path: &str) -> anyhow::Result<()> {
    use anyhow::{bail, ensure, Context};
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::Shell::{SHOpenFolderAndSelectItems, SHParseDisplayName},
    };

    let path = std::path::absolute(path).context("Could not resolve export file path")?;
    ensure!(
        path.metadata()
            .with_context(|| format!("Export file is unavailable: {}", path.display()))?
            .is_file(),
        "Export path is not a file: {}",
        path.display()
    );
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    ensure!(
        !wide.contains(&0),
        "Export file path contains a null character"
    );
    wide.push(0);

    // SAFETY: Windows receives a null-terminated UTF-16 path and valid output
    // pointers. The shell owns the PIDL allocation until CoTaskMemFree. Balance
    // only our successful COM initialization; an existing apartment is reused.
    unsafe {
        let initialized = CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED as u32);
        if initialized < 0 && initialized != RPC_E_CHANGED_MODE {
            bail!(
                "CoInitializeEx failed: HRESULT 0x{:08X}",
                initialized as u32
            );
        }
        let mut item = ptr::null_mut();
        let parsed = SHParseDisplayName(
            wide.as_ptr(),
            ptr::null_mut(),
            &mut item,
            0,
            ptr::null_mut(),
        );
        let result = if parsed < 0 {
            Err(anyhow::anyhow!(
                "SHParseDisplayName failed: HRESULT 0x{:08X}",
                parsed as u32
            ))
        } else if item.is_null() {
            Err(anyhow::anyhow!(
                "Windows returned no shell item for the export file"
            ))
        } else {
            // A fully qualified file PIDL with cidl = 0 opens its parent and
            // selects the file, without constructing an Explorer command line.
            let opened = SHOpenFolderAndSelectItems(item, 0, ptr::null(), 0);
            if opened < 0 {
                Err(anyhow::anyhow!(
                    "SHOpenFolderAndSelectItems failed: HRESULT 0x{:08X}",
                    opened as u32
                ))
            } else {
                Ok(())
            }
        };
        CoTaskMemFree(item.cast());
        if initialized >= 0 {
            CoUninitialize();
        }
        result.with_context(|| format!("Could not reveal export file: {}", path.display()))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn reveal_file(_path: &str) -> anyhow::Result<()> {
    anyhow::bail!("Opening the export folder is supported only on Windows")
}
