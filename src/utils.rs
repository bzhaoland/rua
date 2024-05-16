use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

use anyhow::{Error, Result};
use libc;

/// Get current machine's hostname
#[allow(dead_code)]
pub fn get_hostname() -> Result<OsString> {
    let hostname_bufsize = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) } as usize;
    let mut hostname_buf = vec![0; hostname_bufsize + 1];
    let retcode = unsafe {
        libc::gethostname(
            hostname_buf.as_mut_ptr() as *mut libc::c_char,
            hostname_buf.len(),
        )
    };
    if retcode != 0 {
        return Err(Error::msg("Failed to get hostname"));
    }

    let end = hostname_buf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(hostname_buf.len());
    hostname_buf.resize(end, 0);
    Ok(OsString::from_vec(hostname_buf))
}
