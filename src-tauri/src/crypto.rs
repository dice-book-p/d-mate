use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// 키 쌍 생성 (앱 최초 설정 시)
pub fn generate_keypair() -> (Vec<u8>, String) {
    let secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let public = PublicKey::from(&secret);
    let private_bytes = secret.to_bytes().to_vec();
    let public_b64 = B64.encode(public.as_bytes());
    (private_bytes, public_b64)
}

/// 메시지 암호화 (발신 측)
/// 1. EphemeralSecret 생성
/// 2. 상대방 공개키와 ECDH → shared_secret
/// 3. AES-256-GCM으로 암호화
/// 반환: (ciphertext_b64, ephemeral_pubkey_b64, nonce_b64)
pub fn encrypt_message(
    recipient_pubkey_b64: &str,
    plaintext: &str,
) -> Result<(String, String, String), String> {
    let recipient_bytes: [u8; 32] = B64.decode(recipient_pubkey_b64)
        .map_err(|e| format!("공개키 디코딩 실패: {}", e))?
        .try_into()
        .map_err(|_| "공개키 길이 오류".to_string())?;
    let recipient_pubkey = PublicKey::from(recipient_bytes);

    let ephemeral_secret = EphemeralSecret::random_from_rng(rand::rngs::OsRng);
    let ephemeral_pubkey = PublicKey::from(&ephemeral_secret);

    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_pubkey);

    let key = Key::<Aes256Gcm>::from_slice(shared_secret.as_bytes());
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("암호화 실패: {}", e))?;

    Ok((
        B64.encode(&ciphertext),
        B64.encode(ephemeral_pubkey.as_bytes()),
        B64.encode(&nonce_bytes),
    ))
}

/// 메시지 복호화 (수신 측)
/// 1. 내 private key + ephemeral pubkey → ECDH → shared_secret
/// 2. AES-256-GCM 복호화
pub fn decrypt_message(
    my_private_key: &[u8],
    ephemeral_pubkey_b64: &str,
    nonce_b64: &str,
    ciphertext_b64: &str,
) -> Result<String, String> {
    let my_secret = StaticSecret::from(
        <[u8; 32]>::try_from(my_private_key)
            .map_err(|_| "개인키 길이 오류".to_string())?
    );

    let eph_bytes: [u8; 32] = B64.decode(ephemeral_pubkey_b64)
        .map_err(|e| format!("ephemeral key 디코딩: {}", e))?
        .try_into()
        .map_err(|_| "ephemeral key 길이 오류".to_string())?;
    let eph_pubkey = PublicKey::from(eph_bytes);

    let shared_secret = my_secret.diffie_hellman(&eph_pubkey);

    let key = Key::<Aes256Gcm>::from_slice(shared_secret.as_bytes());
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes: [u8; 12] = B64.decode(nonce_b64)
        .map_err(|e| format!("nonce 디코딩: {}", e))?
        .try_into()
        .map_err(|_| "nonce 길이 오류".to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = B64.decode(ciphertext_b64)
        .map_err(|e| format!("암호문 디코딩: {}", e))?;

    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "복호화 실패 (키 불일치)".to_string())?;

    String::from_utf8(plaintext)
        .map_err(|e| format!("UTF-8 변환 실패: {}", e))
}
