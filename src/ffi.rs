#![cfg(feature = "ffi")]

use crate::{FilebinProvider, FileProvider, JumpshareProvider, ProviderError, TempShProvider, TmpfilesProvider, UploadedFile};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

fn run_blocking<F, T>(future: F) -> Result<T, ProviderError>
where
    F: std::future::Future<Output = Result<T, ProviderError>>,
{
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ProviderError::Api(format!("Runtime error: {}", e)))?;
    rt.block_on(future)
}

fn cstr_to_string(ptr: *const c_char) -> Result<String, ProviderError> {
    if ptr.is_null() {
        return Err(ProviderError::Parse("Null pointer".to_string()));
    }

    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str()
        .map(|s| s.to_string())
        .map_err(|e| ProviderError::Parse(format!("Invalid UTF-8: {}", e)))
}

#[no_mangle]
pub extern "C" fn srapi_filebin_new() -> *mut FilebinProvider {
    Box::into_raw(Box::new(FilebinProvider::new()))
}

#[no_mangle]
pub extern "C" fn srapi_provider_free(ptr: *mut FilebinProvider) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn srapi_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn srapi_filebin_create_bin(ptr: *mut FilebinProvider) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let provider = unsafe { &*ptr };
    match run_blocking(provider.create_bin()) {
        Ok(bin_id) => CString::new(bin_id).ok().map_or(std::ptr::null_mut(), |s| s.into_raw()),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn srapi_filebin_upload_file(
    ptr: *mut FilebinProvider,
    bin_id: *const c_char,
    file_path: *const c_char,
    filename: *const c_char,
) -> bool {
    if ptr.is_null() {
        return false;
    }

    let provider = unsafe { &*ptr };
    let bin_id = match cstr_to_string(bin_id) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let file_path = match cstr_to_string(file_path) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let filename = if filename.is_null() {
        None
    } else {
        cstr_to_string(filename).ok()
    };

    let result = run_blocking(async {
        let path = Path::new(&file_path);
        let file_handle = tokio::fs::File::open(path).await?;
        let metadata = file_handle.metadata().await?;
        let name = match filename {
            Some(v) => v,
            None => path
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| ProviderError::Parse("Invalid file name".to_string()))?
                .to_string(),
        };

        let stream = tokio_util::codec::FramedRead::new(
            file_handle,
            tokio_util::codec::BytesCodec::new(),
        );
        let body = reqwest::Body::wrap_stream(stream);

        provider.upload_file(&bin_id, &name, body, metadata.len()).await?;
        Ok(())
    });

    result.is_ok()
}

#[no_mangle]
pub extern "C" fn srapi_tempsh_new() -> *mut TempShProvider {
    Box::into_raw(Box::new(TempShProvider::new()))
}

#[no_mangle]
pub extern "C" fn srapi_tempsh_free(ptr: *mut TempShProvider) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn srapi_tempsh_upload_file(
    ptr: *mut TempShProvider,
    file_path: *const c_char,
    filename: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let provider = unsafe { &*ptr };
    let file_path = match cstr_to_string(file_path) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    let filename = if filename.is_null() {
        None
    } else {
        cstr_to_string(filename).ok()
    };

    let result = run_blocking(async {
        let path = Path::new(&file_path);
        let name = match filename {
            Some(v) => v,
            None => path
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| ProviderError::Parse("Invalid file name".to_string()))?
                .to_string(),
        };
        let bytes = tokio::fs::read(path).await?;
        provider.upload_bytes(&name, bytes).await
    });

    match result {
        Ok(uploaded) => CString::new(uploaded.url).ok().map_or(std::ptr::null_mut(), |s| s.into_raw()),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn srapi_tempsh_get_info(
    ptr: *mut TempShProvider,
    url: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let provider = unsafe { &*ptr };
    let url = match cstr_to_string(url) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    match run_blocking(provider.get_file_info(&url)) {
        Ok(info) => to_json_string(info),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn srapi_tmpfiles_new() -> *mut TmpfilesProvider {
    Box::into_raw(Box::new(TmpfilesProvider::new()))
}

#[no_mangle]
pub extern "C" fn srapi_tmpfiles_free(ptr: *mut TmpfilesProvider) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn srapi_tmpfiles_upload_file(
    ptr: *mut TmpfilesProvider,
    file_path: *const c_char,
    filename: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let provider = unsafe { &*ptr };
    let file_path = match cstr_to_string(file_path) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    let filename = if filename.is_null() {
        None
    } else {
        cstr_to_string(filename).ok()
    };

    let result = run_blocking(async {
        let path = Path::new(&file_path);
        let name = match filename {
            Some(v) => v,
            None => path
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| ProviderError::Parse("Invalid file name".to_string()))?
                .to_string(),
        };
        let bytes = tokio::fs::read(path).await?;
        provider.upload_bytes(&name, bytes).await
    });

    match result {
        Ok(uploaded) => CString::new(uploaded.url).ok().map_or(std::ptr::null_mut(), |s| s.into_raw()),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn srapi_tmpfiles_get_info(
    ptr: *mut TmpfilesProvider,
    url: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let provider = unsafe { &*ptr };
    let url = match cstr_to_string(url) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    match run_blocking(provider.get_file_info(&url)) {
        Ok(info) => to_json_string(info),
        Err(_) => std::ptr::null_mut(),
    }
}

fn to_json_string(info: UploadedFile) -> *mut c_char {
    let expires = info.expires_at.unwrap_or_default();
    let json = format!(
        "{{\"url\":\"{}\",\"filename\":\"{}\",\"size\":{},\"content_type\":\"{}\",\"expires_at\":\"{}\"}}",
        escape_json(&info.url),
        escape_json(&info.filename),
        info.size,
        escape_json(&info.content_type),
        escape_json(&expires),
    );

    CString::new(json).ok().map_or(std::ptr::null_mut(), |s| s.into_raw())
}

fn escape_json(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[no_mangle]
pub extern "C" fn srapi_jumpshare_new() -> *mut JumpshareProvider {
    Box::into_raw(Box::new(JumpshareProvider::new()))
}

#[no_mangle]
pub extern "C" fn srapi_jumpshare_free(ptr: *mut JumpshareProvider) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn srapi_jumpshare_upload_file(
    ptr: *mut JumpshareProvider,
    file_path: *const c_char,
    filename: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let provider = unsafe { &*ptr };
    let file_path = match cstr_to_string(file_path) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    let filename = if filename.is_null() {
        None
    } else {
        cstr_to_string(filename).ok()
    };

    let result = run_blocking(async {
        let path = Path::new(&file_path);
        let name = match filename {
            Some(v) => v,
            None => path
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| ProviderError::Parse("Invalid file name".to_string()))?
                .to_string(),
        };
        let bytes = tokio::fs::read(path).await?;
        provider.upload_bytes(&name, bytes).await
    });

    match result {
        Ok(uploaded) => CString::new(uploaded.url).ok().map_or(std::ptr::null_mut(), |s| s.into_raw()),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn srapi_jumpshare_get_info(
    ptr: *mut JumpshareProvider,
    url: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let provider = unsafe { &*ptr };
    let url = match cstr_to_string(url) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    match run_blocking(provider.get_file_info(&url)) {
        Ok(info) => to_json_string(info),
        Err(_) => std::ptr::null_mut(),
    }
}
