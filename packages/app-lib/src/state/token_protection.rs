use base64::{Engine, prelude::BASE64_STANDARD};

#[cfg(windows)]
fn crypt(bytes: &[u8], protect: bool) -> crate::Result<Vec<u8>> {
    use windows::Win32::{
        Foundation::{HLOCAL, LocalFree},
        Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData,
            CryptUnprotectData,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len().try_into()?,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        if protect {
            CryptProtectData(
                &input,
                windows::core::w!("gecl Minecraft credentials"),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )?;
        } else {
            CryptUnprotectData(
                &input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )?;
        }
        let result =
            std::slice::from_raw_parts(output.pbData, output.cbData as usize)
                .to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(result)
    }
}

#[cfg(not(windows))]
fn crypt(_bytes: &[u8], _protect: bool) -> crate::Result<Vec<u8>> {
    anyhow::bail!("此 gecl 构建的凭据保护仅支持 Windows");
}

pub(super) fn protect(token: &str) -> crate::Result<String> {
    Ok(format!(
        "dpapi:{}",
        BASE64_STANDARD.encode(crypt(token.as_bytes(), true)?)
    ))
}

pub(super) fn unprotect(token: &str) -> crate::Result<String> {
    if let Some(encoded) = token.strip_prefix("dpapi:") {
        Ok(String::from_utf8(crypt(
            &BASE64_STANDARD.decode(encoded)?,
            false,
        )?)?)
    } else {
        Ok(token.to_owned())
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn gecl_current_windows_user_round_trip_and_corruption() {
        let encrypted = super::protect("gecl-test-token").unwrap();
        assert!(!encrypted.contains("gecl-test-token"));
        assert_eq!(super::unprotect(&encrypted).unwrap(), "gecl-test-token");
        assert!(super::unprotect("dpapi:broken").is_err());
    }
}
