// Build mcuboot as a library, based on the requested features.

extern crate cc;

use std::env;
use std::fs;
use std::io;
use std::path::Path;

fn main() {
    // Feature flags.
    let sig_rsa = env::var("CARGO_FEATURE_SIG_RSA").is_ok();
    let sig_rsa3072 = env::var("CARGO_FEATURE_SIG_RSA3072").is_ok();
    let sig_ecdsa = env::var("CARGO_FEATURE_SIG_ECDSA").is_ok();
    let sig_ed25519 = env::var("CARGO_FEATURE_SIG_ED25519").is_ok();
    let overwrite_only = env::var("CARGO_FEATURE_OVERWRITE_ONLY").is_ok();
    let validate_primary_slot =
                  env::var("CARGO_FEATURE_VALIDATE_PRIMARY_SLOT").is_ok();
    let enc_rsa = env::var("CARGO_FEATURE_ENC_RSA").is_ok();
    let enc_kw = env::var("CARGO_FEATURE_ENC_KW").is_ok();
    let bootstrap = env::var("CARGO_FEATURE_BOOTSTRAP").is_ok();
    let multiimage = env::var("CARGO_FEATURE_MULTIIMAGE").is_ok();

    let mut conf = cc::Build::new();
    conf.define("__BOOTSIM__", None);
    conf.define("MCUBOOT_HAVE_LOGGING", None);
    conf.define("MCUBOOT_USE_FLASH_AREA_GET_SECTORS", None);
    conf.define("MCUBOOT_HAVE_ASSERT_H", None);
    conf.define("MCUBOOT_MAX_IMG_SECTORS", Some("128"));
    conf.define("MCUBOOT_IMAGE_NUMBER", Some(if multiimage { "2" } else { "1" }));

    if bootstrap {
        conf.define("MCUBOOT_BOOTSTRAP", None);
    }

    if validate_primary_slot {
        conf.define("MCUBOOT_VALIDATE_PRIMARY_SLOT", None);
    }

    // Currently no more than one sig type can be used simultaneously.
    if vec![sig_rsa, sig_rsa3072, sig_ecdsa, sig_ed25519].iter()
        .fold(0, |sum, &v| sum + v as i32) > 1 {
        panic!("mcuboot does not support more than one sig type at the same time");
    }

    if sig_rsa || sig_rsa3072 {
        conf.define("MCUBOOT_SIGN_RSA", None);
        // The Kconfig style defines must be added here as well because
        // they are used internally by "config-rsa.h"
        if sig_rsa {
            conf.define("MCUBOOT_SIGN_RSA_LEN", "2048");
            conf.define("CONFIG_BOOT_SIGNATURE_TYPE_RSA_2048", None);
        } else {
            conf.define("MCUBOOT_SIGN_RSA_LEN", "3072");
            conf.define("CONFIG_BOOT_SIGNATURE_TYPE_RSA_3072", None);
        }
        conf.define("MCUBOOT_USE_MBED_TLS", None);

        conf.include("mbedtls/include");
        conf.file("mbedtls/library/sha256.c");
        conf.file("csupport/keys.c");

        conf.file("mbedtls/library/rsa.c");
        conf.file("mbedtls/library/bignum.c");
        conf.file("mbedtls/library/platform.c");
        conf.file("mbedtls/library/platform_util.c");
        conf.file("mbedtls/library/asn1parse.c");
    } else if sig_ecdsa {
        conf.define("MCUBOOT_SIGN_EC256", None);
        conf.define("MCUBOOT_USE_TINYCRYPT", None);

        if !enc_kw {
            conf.include("../../ext/mbedtls/include");
        }
        conf.include("../../ext/tinycrypt/lib/include");

        conf.file("csupport/keys.c");

        conf.file("../../ext/tinycrypt/lib/source/utils.c");
        conf.file("../../ext/tinycrypt/lib/source/sha256.c");
        conf.file("../../ext/tinycrypt/lib/source/ecc.c");
        conf.file("../../ext/tinycrypt/lib/source/ecc_dsa.c");
        conf.file("../../ext/tinycrypt/lib/source/ecc_platform_specific.c");

        conf.file("../../ext/mbedtls/src/platform_util.c");
        conf.file("../../ext/mbedtls/src/asn1parse.c");
    } else if sig_ed25519 {
        conf.define("MCUBOOT_SIGN_ED25519", None);
        conf.define("MCUBOOT_USE_MBED_TLS", None);

        conf.include("mbedtls/include");
        conf.file("mbedtls/library/sha256.c");
        conf.file("mbedtls/library/sha512.c");
        conf.file("csupport/keys.c");
        conf.file("../../ext/fiat/src/curve25519.c");
        conf.file("mbedtls/library/platform.c");
        conf.file("mbedtls/library/platform_util.c");
        conf.file("mbedtls/library/asn1parse.c");
    } else {
        // Neither signature type, only verify sha256. The default
        // configuration file bundled with mbedTLS is sufficient.
        conf.define("MCUBOOT_USE_MBED_TLS", None);
        conf.include("mbedtls/include");
        conf.file("mbedtls/library/sha256.c");
    }

    if overwrite_only {
        conf.define("MCUBOOT_OVERWRITE_ONLY", None);
        conf.define("MCUBOOT_OVERWRITE_ONLY_FAST", None);
    }

    if enc_rsa {
        conf.define("MCUBOOT_ENCRYPT_RSA", None);
        conf.define("MCUBOOT_ENC_IMAGES", None);
        conf.define("MCUBOOT_USE_MBED_TLS", None);

        conf.file("../../boot/bootutil/src/encrypted.c");
        conf.file("csupport/keys.c");

        conf.include("mbedtls/include");
        conf.file("mbedtls/library/sha256.c");

        conf.file("mbedtls/library/platform.c");
        conf.file("mbedtls/library/platform_util.c");
        conf.file("mbedtls/library/rsa.c");
        conf.file("mbedtls/library/rsa_internal.c");
        conf.file("mbedtls/library/md.c");
        conf.file("mbedtls/library/md_wrap.c");
        conf.file("mbedtls/library/aes.c");
        conf.file("mbedtls/library/bignum.c");
        conf.file("mbedtls/library/asn1parse.c");
    }

    if enc_kw {
        conf.define("MCUBOOT_ENCRYPT_KW", None);
        conf.define("MCUBOOT_ENC_IMAGES", None);

        conf.file("../../boot/bootutil/src/encrypted.c");
        conf.file("csupport/keys.c");

        if sig_rsa || sig_rsa3072 {
            conf.file("mbedtls/library/sha256.c");
        }

        /* Simulator uses Mbed-TLS to wrap keys */
        conf.include("mbedtls/include");
        conf.file("mbedtls/library/platform.c");
        conf.file("mbedtls/library/platform_util.c");
        conf.file("mbedtls/library/nist_kw.c");
        conf.file("mbedtls/library/cipher.c");
        conf.file("mbedtls/library/cipher_wrap.c");
        conf.file("mbedtls/library/aes.c");

        if sig_ecdsa {
            conf.define("MCUBOOT_USE_TINYCRYPT", None);

            conf.include("../../ext/tinycrypt/lib/include");

            conf.file("../../ext/tinycrypt/lib/source/utils.c");
            conf.file("../../ext/tinycrypt/lib/source/sha256.c");
            conf.file("../../ext/tinycrypt/lib/source/aes_encrypt.c");
            conf.file("../../ext/tinycrypt/lib/source/aes_decrypt.c");
        }

        if sig_ed25519 {
            panic!("ed25519 does not support image encryption with KW yet");
        }
    }

    if sig_rsa && enc_kw {
        conf.define("MBEDTLS_CONFIG_FILE", Some("<config-rsa-kw.h>"));
    } else if sig_rsa || sig_rsa3072 || enc_rsa {
        conf.define("MBEDTLS_CONFIG_FILE", Some("<config-rsa.h>"));
    } else if sig_ecdsa && !enc_kw {
        conf.define("MBEDTLS_CONFIG_FILE", Some("<config-asn1.h>"));
    } else if sig_ed25519 {
        conf.define("MBEDTLS_CONFIG_FILE", Some("<config-ed25519.h>"));
    } else if enc_kw {
        conf.define("MBEDTLS_CONFIG_FILE", Some("<config-kw.h>"));
    }

    conf.file("../../boot/bootutil/src/image_validate.c");
    if sig_rsa || sig_rsa3072 {
        conf.file("../../boot/bootutil/src/image_rsa.c");
    } else if sig_ecdsa {
        conf.file("../../boot/bootutil/src/image_ec256.c");
    } else if sig_ed25519 {
        conf.file("../../boot/bootutil/src/image_ed25519.c");
    }
    conf.file("../../boot/bootutil/src/loader.c");
    conf.file("../../boot/bootutil/src/caps.c");
    conf.file("../../boot/bootutil/src/bootutil_misc.c");
    conf.file("../../boot/bootutil/src/tlv.c");
    conf.file("csupport/run.c");
    conf.include("../../boot/bootutil/include");
    conf.include("csupport");
    conf.include("../../boot/zephyr/include");
    conf.debug(true);
    conf.flag("-Wall");
    conf.flag("-Werror");

    // FIXME: travis-ci still uses gcc 4.8.4 which defaults to std=gnu90.
    // It has incomplete std=c11 and std=c99 support but std=c99 was checked
    // to build correctly so leaving it here to updated in the future...
    conf.flag("-std=c99");

    conf.compile("libbootutil.a");

    walk_dir("../../boot").unwrap();
    walk_dir("../../ext/tinycrypt/lib/source").unwrap();
    walk_dir("../../ext/mbedtls").unwrap();
    walk_dir("csupport").unwrap();
    walk_dir("mbedtls/include").unwrap();
    walk_dir("mbedtls/library").unwrap();
}

// Output the names of all files within a directory so that Cargo knows when to rebuild.
fn walk_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for ent in fs::read_dir(path.as_ref())? {
        let ent = ent?;
        let p = ent.path();
        if p.is_dir() {
            walk_dir(p)?;
        } else {
            // Note that non-utf8 names will fail.
            let name = p.to_str().unwrap();
            if name.ends_with(".c") || name.ends_with(".h") {
                println!("cargo:rerun-if-changed={}", name);
            }
        }
    }

    Ok(())
}
