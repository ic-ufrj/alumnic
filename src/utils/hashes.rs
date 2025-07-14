use base64::prelude::*;
use encoding::all::UTF_16LE;
use encoding::{EncoderTrap, Encoding};
use md4::Md4;
use rand::Rng;
use secrecy::{ExposeSecret, SecretString};
use sha1::{Digest, Sha1};
use zeroize::Zeroize;

/// Computa a hash usada pelo Samba de uma String.
///
/// # Examples
///
/// ```
/// # use alumnic::utils::hashes::hash_nt;
/// # use secrecy::ExposeSecret;
/// assert_eq!(
///     hash_nt(&"12345678".to_string().into()).expose_secret(),
///     "259745CB123A52AA2E693AAACCA2DB52",
/// );
pub fn hash_nt(passwd: &SecretString) -> SecretString {
    let mut passwd_utf16le = UTF_16LE
        .encode(passwd.expose_secret(), EncoderTrap::Strict)
        .unwrap();
    let mut hasher = Md4::new();
    hasher.update(&passwd_utf16le);
    let r: SecretString = hex::encode_upper(hasher.finalize()).into();

    passwd_utf16le.zeroize();

    r
}

/// Computa a hash SSHA usada para o login nos laboratórios
///
/// **Essa hash é considerada insegura há um tempo. Provavelmente é possível
/// passar a usar bcrypt em breve.**
///
/// # Examples
///
/// ```
/// # use alumnic::utils::hashes::{hash_ssha, compare_ssha};
/// # use secrecy::ExposeSecret;
/// assert!(compare_ssha(
///     &"12345678".to_string().into(),
///     &hash_ssha(&"12345678".to_string().into()),
/// ));
/// ```
pub fn hash_ssha(passwd: &SecretString) -> SecretString {
    let mut salt = [0u8; 4];
    rand::rng().fill(&mut salt);

    let r = hash_ssha_with_salt(passwd, &salt);

    salt.zeroize();

    r
}

fn hash_ssha_with_salt(passwd: &SecretString, salt: &[u8; 4]) -> SecretString {
    let mut hasher = Sha1::new();
    hasher.update(passwd.expose_secret().as_bytes());
    hasher.update(&salt);
    let mut hash = hasher.finalize();

    let mut salted = BASE64_STANDARD.encode([hash.as_slice(), salt].concat());

    let r: SecretString = format!("{}{}", "{SSHA}", salted).into();

    hash.zeroize();
    salted.zeroize();

    r
}

/// Verifica se a senha é a senha hasheada.
///
/// # Panics
///
/// - quando a hash não começa com `{SSHA}`;
/// - quando a hash não é base64 válido; e
/// - quando a hash não tem 24 bytes após decodificar o base64.
pub fn compare_ssha(passwd: &SecretString, hash: &SecretString) -> bool {
    let mut hash_unbased = BASE64_STANDARD
        .decode(hash.expose_secret().strip_prefix("{SSHA}").unwrap())
        .unwrap();

    let (_, salt) = hash_unbased.split_at(20);
    let mut salt_fixed = <[u8; 4]>::try_from(salt).unwrap();

    let new_hash = hash_ssha_with_salt(passwd, &salt_fixed);

    hash_unbased.zeroize();
    salt_fixed.zeroize();

    new_hash.expose_secret() == hash.expose_secret()
}
