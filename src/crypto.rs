

// use std::process::Command;
use openssl::{
    rsa::{Rsa, Padding},
    encrypt::{Encrypter, Decrypter},
    pkey::{PKey, Private, Public},
};

enum Key {
    Public(PKey<Public>),
    Private(PKey<Private>),
}
pub struct CryptoHandler {
    key: Key,
}
impl CryptoHandler {
    pub fn new() -> Self {
       let rsa = Rsa::generate(2048).unwrap();
       let pkey = PKey::from_rsa(rsa).unwrap();
        Self {key: Key::Private(pkey)}
    }

    pub fn new_from_public_key_pem(key: String) -> Self {
        Self {key: Key::Public(PKey::public_key_from_pem(&key.as_bytes()).unwrap())}
    }

    // generate public key from private key
    pub fn public_key(&self) -> String {
        match &self.key {
            Key::Private(key) => String::from_utf8(key.public_key_to_pem().unwrap()).unwrap(),
            _ => panic!("no private key available")
        }
    }

    pub fn encrypt(&self, string: String) -> String {
        match &self.key {
            Key::Public(key) => {
                // encrypt string into hex string
                let mut encrypter = Encrypter::new(key).unwrap();            
                encrypter.set_rsa_padding(Padding::PKCS1).unwrap();
                let mut encrypted = {
                    let buffer_len = encrypter.encrypt_len(&string.as_bytes()).unwrap();
                    vec![0; buffer_len]
                };
                let encrypted_len = encrypter.encrypt(&string.as_bytes(), &mut encrypted).unwrap();
                encrypted.truncate(encrypted_len);
                let encrypted_str = {
                    let mut string = String::new();
                    for ele in &encrypted {
                        let mut strr = format!("{:x?}", ele);
                        if strr.len() != 2 {strr = "0".to_owned()+&strr}
                        string += &strr;
                    }
                    string
                };
                encrypted_str
            },
            _ => panic!("cant encrypt without public key"),
        }
    }

    pub fn decrypt(&self, string: String) -> String {
        match &self.key {    
            Key::Private(key) => {
                // decrypt from the encrypted hex string
                let mut decrypter = Decrypter::new(key).unwrap();
                decrypter.set_rsa_padding(Padding::PKCS1).unwrap();         
                let encrypted: Vec<u8> = {
                    (0..string.len()).step_by(2)
                        .map(|i| u8::from_str_radix(
                            &string[i..i+2], 16).unwrap()
                        )
                        .collect()
                };
                let mut decrypted = {
                    let buffer_len = decrypter.decrypt_len(&encrypted).unwrap();
                    vec![0; buffer_len]
                };
                let decrypted_len = decrypter.decrypt(&encrypted, &mut decrypted).unwrap();
                decrypted.truncate(decrypted_len);
                String::from_utf8(decrypted.clone()).unwrap()
            },
            _ => panic!("cant decrypt without a private key"),
        }
    }
}
