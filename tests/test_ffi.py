#!/usr/bin/env python3
import ctypes
import os
import sys
from pathlib import Path


def find_shared_lib() -> Path:
    candidates = [
        Path("target/debug/libsrapi_rs.so"),
        Path("target/release/libsrapi_rs.so"),
    ]
    for path in candidates:
        if path.exists():
            return path
    raise FileNotFoundError("Could not find libsrapi_rs.so in target/debug or target/release")


def main() -> int:
    lib_path = find_shared_lib()
    lib = ctypes.CDLL(str(lib_path))

    # Define signatures
    lib.srapi_filebin_new.restype = ctypes.c_void_p
    lib.srapi_provider_free.argtypes = [ctypes.c_void_p]
    lib.srapi_provider_free.restype = None

    lib.srapi_filebin_create_bin.argtypes = [ctypes.c_void_p]
    lib.srapi_filebin_create_bin.restype = ctypes.c_void_p

    lib.srapi_string_free.argtypes = [ctypes.c_char_p]
    lib.srapi_string_free.restype = None

    lib.srapi_filebin_upload_file.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.c_char_p,
    ]
    lib.srapi_filebin_upload_file.restype = ctypes.c_bool

    lib.srapi_tempsh_new.restype = ctypes.c_void_p
    lib.srapi_tempsh_free.argtypes = [ctypes.c_void_p]
    lib.srapi_tempsh_free.restype = None
    lib.srapi_tempsh_upload_file.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.c_char_p,
    ]
    lib.srapi_tempsh_upload_file.restype = ctypes.c_void_p

    lib.srapi_jumpshare_new.restype = ctypes.c_void_p
    lib.srapi_jumpshare_free.argtypes = [ctypes.c_void_p]
    lib.srapi_jumpshare_free.restype = None
    lib.srapi_jumpshare_upload_file.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.c_char_p,
    ]
    lib.srapi_jumpshare_upload_file.restype = ctypes.c_void_p
    lib.srapi_jumpshare_get_info.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
    ]
    lib.srapi_jumpshare_get_info.restype = ctypes.c_void_p
    lib.srapi_tempsh_get_info.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    lib.srapi_tempsh_get_info.restype = ctypes.c_void_p

    lib.srapi_tmpfiles_new.restype = ctypes.c_void_p
    lib.srapi_tmpfiles_free.argtypes = [ctypes.c_void_p]
    lib.srapi_tmpfiles_free.restype = None
    lib.srapi_tmpfiles_upload_file.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.c_char_p,
    ]
    lib.srapi_tmpfiles_upload_file.restype = ctypes.c_void_p
    lib.srapi_tmpfiles_get_info.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    lib.srapi_tmpfiles_get_info.restype = ctypes.c_void_p

    provider = lib.srapi_filebin_new()
    if not provider:
        print("Failed to create provider")
        return 1

    try:
        # Only hit live network if explicitly enabled
        live = os.getenv("SRAPI_LIVE_TEST") == "1"
        if live:
            bin_ptr = lib.srapi_filebin_create_bin(provider)
            if not bin_ptr:
                print("FFI create_bin failed")
                return 1
            bin_id = ctypes.string_at(bin_ptr).decode("utf-8")
            print("Created bin:", bin_id)
            lib.srapi_string_free(ctypes.c_char_p(bin_ptr))

            test_path = Path("/tmp/srapi_ffi_test.txt")
            test_path.write_text("hello from ffi\n")

            # temp.sh live upload + info
            tempsh = lib.srapi_tempsh_new()
            if not tempsh:
                print("FFI temp.sh provider create failed")
                return 1
            try:
                url_ptr = lib.srapi_tempsh_upload_file(
                    tempsh,
                    str(test_path).encode("utf-8"),
                    b"ffi_test.txt",
                )
                if not url_ptr:
                    print("FFI temp.sh upload failed")
                    return 1
                url = ctypes.string_at(url_ptr).decode("utf-8")
                print("Temp.sh URL:", url)
                lib.srapi_string_free(ctypes.c_char_p(url_ptr))

                info_ptr = lib.srapi_tempsh_get_info(tempsh, url.encode("utf-8"))
                if not info_ptr:
                    print("FFI temp.sh info failed")
                    return 1
                info_json = ctypes.string_at(info_ptr).decode("utf-8")
                print("Temp.sh info:", info_json)
                lib.srapi_string_free(ctypes.c_char_p(info_ptr))
            finally:
                lib.srapi_tempsh_free(tempsh)

            # tmpfiles live upload + info
            tmpfiles = lib.srapi_tmpfiles_new()
            if not tmpfiles:
                print("FFI tmpfiles provider create failed")
                return 1
            try:
                url_ptr = lib.srapi_tmpfiles_upload_file(
                    tmpfiles,
                    str(test_path).encode("utf-8"),
                    b"ffi_test.txt",
                )
                if not url_ptr:
                    print("FFI tmpfiles upload failed")
                    return 1
                url = ctypes.string_at(url_ptr).decode("utf-8")
                print("Tmpfiles URL:", url)
                lib.srapi_string_free(ctypes.c_char_p(url_ptr))

                info_ptr = lib.srapi_tmpfiles_get_info(tmpfiles, url.encode("utf-8"))
                if not info_ptr:
                    print("FFI tmpfiles info failed")
                    return 1
                info_json = ctypes.string_at(info_ptr).decode("utf-8")
                print("Tmpfiles info:", info_json)
                lib.srapi_string_free(ctypes.c_char_p(info_ptr))
            finally:
                lib.srapi_tmpfiles_free(tmpfiles)

            # Jumpshare
            jumpshare = lib.srapi_jumpshare_new()
            if not jumpshare:
                print("FFI jumpshare provider create failed")
                return 1
            try:
                print("Testing Jumpshare provider...")
                url_ptr = lib.srapi_jumpshare_upload_file(
                    jumpshare,
                    str(test_path).encode("utf-8"),
                    b"ffi_test.txt",
                )
                if not url_ptr:
                    # Jumpshare might fail due to rate limits or IP bans (anonymous usage)
                    # We print failure but maybe don't fail the whole test if it's external?
                    # The prompt says "anyway do the jumpshare provider now", so expectation is it works.
                    print("FFI jumpshare upload failed")
                    return 1
                url = ctypes.string_at(url_ptr).decode("utf-8")
                print("Jumpshare URL:", url)
                lib.srapi_string_free(ctypes.c_char_p(url_ptr))
            finally:
                lib.srapi_jumpshare_free(jumpshare)

        else:
            print("FFI load OK (set SRAPI_LIVE_TEST=1 to hit live create_bin)")
    finally:
        lib.srapi_provider_free(provider)

    return 0


if __name__ == "__main__":
    sys.exit(main())
